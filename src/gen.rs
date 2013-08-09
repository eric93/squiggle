use program::*;

use std::cell::Cell;
use std::comm;
use std::comm::{Port, Chan};
use std::hashmap::HashSet;
use std::rand::{Rng, RngUtil, IsaacRng};

pub trait Generator {
    pub fn gen_sym(&mut self) -> Id;
    pub fn gen_expr(&mut self) -> Expr;
    pub fn gen_prog(&mut self) -> Program;

    pub fn get_sym(&mut self) -> Id;
}

pub enum GenMsg {
    Reset(u8, ~HashSet<Operator>, ~[(u64, u64)]),
    Generate(Chan<~Program>),
    Exit,
}

type Scope = ~[Id];

struct ScopeStack {
    stack: ~[Scope],
}

pub struct NaiveGen(Chan<GenMsg>);

pub struct NaiveGenState {
    rng: IsaacRng,
    scopes: ScopeStack,
    next_symbol: u8,
    max_size: u8,
    operations: ~HashSet<Operator>,
    constraints: ~[(u64, u64)],
    size: u8,
}

impl NaiveGen {
    pub fn new(max_size: u8, operations: ~HashSet<Operator>,
               constraints: ~[(u64, u64)]) -> NaiveGen {
        let (port, chan) = comm::stream();

        let port = Cell::new(port);
        do spawn {
            NaiveGen::generate(port.take());
        }

        chan.send(Reset(max_size, operations, constraints));
        NaiveGen(chan)
    }

    // FIXME: this isn't right anymore. maybe it's not needed
    pub fn reset(&mut self) {
        (**self).send(Reset(30, ~HashSet::new(), ~[]));
    }

    pub fn next(&mut self) -> ~Program {
        let (port, chan) = comm::stream();
        (**self).send(Generate(chan));
        port.recv()
    }

    fn generate(port: Port<GenMsg>) {
        let mut gen = NaiveGenState::new();
        loop {
            match port.try_recv() {
                None => break,
                Some(Exit) => break,
                Some(Reset(max_size, operations, constraints)) => {
                    gen.reset(max_size, operations, constraints);
                }
                Some(Generate(chan)) => {
                    let mut i = 0;
                    'newprog: loop {
                        let prog = gen.gen_prog();
                        i += 1;
                        for &(x, y) in gen.constraints.iter() {
                            if prog.eval(x) != y {
                                loop 'newprog;
                            }
                        }
                        // printfln!("genned constrained prog in %u iters", i);
                        chan.send(~prog);
                        break;
                    }
                }
            }
        }
    }
}

impl NaiveGenState {
    pub fn new() -> NaiveGenState {
        NaiveGenState {
            rng: IsaacRng::new(),
            scopes: ScopeStack {
                stack: ~[],
            },
            next_symbol: 0,
            max_size: 30,
            operations: ~HashSet::new(),
            constraints: ~[],
            size: 0,
        }
    }

    pub fn reset(&mut self, max_size: u8, operations: ~HashSet<Operator>, constraints: ~[(u64, u64)]) {
        assert!(max_size >= 3 && max_size <= 30);
        self.scopes = ScopeStack {
            stack: ~[],
        };
        self.next_symbol = 0;
        self.max_size = max_size;
        self.operations = operations;
        self.constraints = constraints;
        self.size = 0;
    }
}

impl Generator for NaiveGenState {
    pub fn gen_sym(&mut self) -> Id {
        let num = self.next_symbol;
        self.next_symbol += 1;
        num as uint
    }

    pub fn gen_expr(&mut self) -> Expr {
        loop {
            let choice = self.rng.gen_uint_range(0, 7);
            match choice {
                0 => {
                    self.size += 1;
                    return Zero;
                }
                1 => {
                    self.size += 1;
                    return One;
                }
                2 => {
                    self.size += 1;
                    return Ident(self.get_sym());
                }
                3 => { // if0
                    let op_ok = self.operations.contains(&Operator_If0);
                    if op_ok && self.size + 4 <= self.max_size {
                        self.size += 1;
                        let test = self.gen_expr();
                        let then = self.gen_expr();
                        let other = self.gen_expr();
                        return If0(~test, ~then, ~other);
                    }
                    loop;
                }
                4 => { // op1
                    // TODO: constrain against operators
                    if self.size + 2 <= self.max_size {
                        self.size += 1;
                        let op = self.rng.gen();
                        let expr = self.gen_expr();
                        return Op1(op, ~expr);
                    }
                    loop;
                }
                5 => { // op2
                    // TODO: constraint against operators
                    if self.size + 3 <= self.max_size {
                        let op = self.rng.gen();
                        let left = self.gen_expr();
                        let right = self.gen_expr();
                        return Op2(op, ~left, ~right);
                    }
                    loop;
                }
                6 => { // fold
                    // TODO: constrain against operators
                    if self.size + 5 <= self.max_size {
                        let foldee = self.gen_expr();
                        let init = self.gen_expr();
                        let next_id = self.gen_sym();
                        let accum_id = self.gen_sym();
                        let scope = ~[next_id.clone(), accum_id.clone()];
                        self.scopes.stack.push(scope);
                        let body = self.gen_expr();
                        self.scopes.stack.pop();
                        return Fold {
                            foldee: ~foldee,
                            init: ~init,
                            next_id: next_id,
                            accum_id: accum_id,
                            body: ~body,
                        };
                    }
                    loop;
                }
                _ => fail!(~"unexpected random value"),
            }
        }
    }

    pub fn gen_prog(&mut self) -> Program {
        let sym = self.gen_sym();
        let scope = ~[sym.clone()];
        self.scopes.stack.push(scope);
        self.size += 1;
        let expr = self.gen_expr();
        self.scopes.stack.clear();
        let ret = Program::new(sym, ~expr);

        ret
    }

    pub fn get_sym(&mut self) -> Id {
        let syms = self.scopes.stack.concat_vec();
        self.rng.choose(syms)
    }
}


#[cfg(test)]
mod tests {
    use extra::test::BenchHarness;
    use super::*;
    use std::hashmap::HashSet;

    #[bench]
    fn bench_gen_prog(bh: &mut BenchHarness) {
        let mut gen = NaiveGen::new(30, ~HashSet::new(), ~[]);
        do bh.iter {
            gen.next();
        }
    }
}

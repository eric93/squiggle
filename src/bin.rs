#[link(name = "squiggle",
       vers = "0.1",
       uuid = "7f09eec3-932c-4096-9c3f-7f0b22c18f2a")];

extern mod std;
extern mod extra;

use std::rand::RngUtil;
use eval::Eval;
use gen::*;
use program::*;
use webapi::{Request, TrainOperators, Empty};

pub mod eval;
pub mod gen;
pub mod program;
pub mod webapi;

fn main() {
    // let status = webapi::Request::get_status();
    // println(status.to_str());

    // let prob = webapi::Request::get_training_problem(3, Empty);

    println(Program::new(~"x", ~Ident(~"x")).to_str());

    // (|x| x << 1 + x)(10)
    printfln!(Program::new(~"x", ~Op2(Plus,
                                      ~Op1(Shl1, ~Ident(~"x")),
                                      ~Ident(~"x"))).eval(10));

    let prog = Program::new(~"x", ~Fold {
            foldee: ~Ident(~"x"),
            init: ~Zero,
            accum_id: ~"y",
            next_id: ~"z",
            body: ~Op2(Or, ~Ident(~"y"), ~Ident(~"z"))
        });

    printfln!(prog.eval(0x1122334455667788))

    // some random programs
    let mut gen = NaiveGen::new(30);
    for _ in range(0, 5) {
        printfln!(gen.gen_prog().to_str());
        gen.reset();
    }

    let prog = Program::new(~"x", ~Op2(Plus, ~Ident(~"x"), ~Ident(~"x")));
    printfln!("matching against %s", prog.to_str());
    println(find_matching(&prog).to_str());
}

fn find_matching(match_against: &Program) -> Program {
    let mut rng = std::rand::task_rng();

    for i in std::iterator::count(0u, 1) {
        let mut gen = NaiveGen::new();
        let prog = gen.gen_prog();

        // say that if it matches on 10000 random numbers, then it's a
        // proper match.
        if range(0, 10000).all(|_|  {
                let i = rng.gen();
                prog.eval(i) == match_against.eval(i)
            }) {
            printfln!("checked %u programs", i);
            return prog;
        }
    }
    fail!()
}

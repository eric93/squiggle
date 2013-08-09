use program::*;

// immutable linked list, lifetime don't work well enough (yet; need
// the ability for 2 lifetimes) for this to avoid @.
struct Scope {
    id: ~str,
    val: u64,
    parent: Option<@Scope>
}

impl Scope {
    fn push(@self, id: ~str, val: u64) -> @Scope {
        @Scope {
            id: id,
            val: val,
            parent: Some(self)
        }
    }

    fn lookup(&self, id: &str) -> u64 {
        if id == self.id {
            self.val
        } else {
            match self.parent {
                Some(p) => p.lookup(id),
                None => fail!("ident %s not found", id)
            }
        }
    }

    fn eval(@self, expr: &Expr) -> u64 {
        match *expr {
            Zero => 0,
            One => 1,
            Ident(ref s) => self.lookup(*s),
            If0(~ref cond, ~ref then, ~ref els) => {
                if self.eval(cond) == 0 {
                    self.eval(then)
                } else {
                    self.eval(els)
                }
            }
            Op1(op, ~ref expr) => {
                let e = self.eval(expr);

                match op {
                    Not => !e,
                    Shl1 => e << 1,
                    Shr1 => e >> 1,
                    Shr4 => e >> 4,
                    Shr16 => e >> 16
                }
            }
            Op2(op, ~ref e1, ~ref e2) => {
                let e1 = self.eval(e1);
                let e2 = self.eval(e2);

                match op {
                    And => e1 & e2,
                    Or => e1 | e2,
                    Xor => e1 ^ e2,
                    Plus => e1 + e2
                }
            }
            Fold {
                foldee: ~ref foldee, init: ~ref init,
                next_id: ref next_id, accum_id: ref accum_id,
                body: ~ref body
            } => {
                let mut foldee = self.eval(foldee);
                let mut accum = self.eval(init);

                for _ in range(0, 8) {
                    let b = foldee & 0xff;
                    foldee >>= 8;

                    let scope = self.push(next_id.clone(), b);
                    let scope = scope.push(accum_id.clone(), accum);

                    accum = scope.eval(body)
                }

                accum
            }
        }
    }
}

pub trait Eval {
    fn eval(&self, val: u64) -> u64;
}
impl Eval for Program {
    fn eval(&self, val: u64) -> u64 {
        (@Scope {
                id: self.id.clone(),
                val: val,
                parent: None
            }).eval(self.expr)
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use program::*;
    use extra::test::BenchHarness;
    use std::rand;

    #[bench]
    fn bench_eval(bh: &mut BenchHarness) {
        // collection of randomly generated programs
        let progs = [
                     Program::new(~"gg", ~Op1(Shr1, ~Ident(~"gg"))),
                     Program::new(~"hg", ~Op1(Shl1, ~Ident(~"hg"))),
                     ];
        let mut rng = rand::rng();

        do bh.iter {
            for p in progs.iter() {
                let p: &Program = p;
                for _ in range(0, 10) {
                    p.eval(rng.gen());
                }
            }
        }
    }
    #[bench]
    fn bench_eval_fold(bh: &mut BenchHarness) {
        let fold_expr = ~Fold {
            foldee: ~Ident(~"x"),
            init: ~Zero,
            next_id: ~"n",
            accum_id: ~"a",
            body: ~Op2(Plus, ~Ident(~"n"), ~Ident(~"a"))
        };
        let prog = Program::new(~"x", fold_expr);
        let mut rng = rand::rng();

        do bh.iter {
            prog.eval(rng.gen());
        }
    }

    #[test]
    fn test_eval() {
        let fold_expr = ~Fold {
            foldee: ~Ident(~"x"),
            init: ~Zero,
            next_id: ~"n",
            accum_id: ~"a",
            body: ~Op2(Plus, ~Ident(~"n"), ~Ident(~"a"))
        };
        let fold_fn = |mut x: u64| {
            let mut accum = 0;
            for _ in range(0, 8) {
                accum += x & 0xff;
                x >>= 8;
            }
            accum
        };

        let progs_fn: ~[(Program, &fn(u64) -> u64)] = ~[
                        (Program::new(~"x", ~Zero), |_| 0),
                        (Program::new(~"x", ~One), |_| 1),
                        (Program::new(~"x", ~Ident(~"x")), |x| x),
                        (Program::new(~"x", ~Op1(Not, ~Ident(~"x"))), |x| !x),
                        (Program::new(~"x", ~Op1(Shl1, ~Ident(~"x"))), |x| x << 1),
                        (Program::new(~"x", ~Op1(Shr1, ~Ident(~"x"))), |x| x >> 1),
                        (Program::new(~"x", ~Op1(Shr4, ~Ident(~"x"))), |x| x >> 4),
                        (Program::new(~"x", ~Op1(Shr16, ~Ident(~"x"))), |x| x >> 16),
                        (Program::new(~"x", ~Op2(And, ~Ident(~"x"), ~One)), |x| x & 1),
                        (Program::new(~"x", ~Op2(Or, ~Ident(~"x"), ~One)), |x| x | 1),
                        (Program::new(~"x", ~Op2(Xor, ~Ident(~"x"), ~One)), |x| x ^ 1),
                        (Program::new(~"x", ~Op2(Plus, ~Ident(~"x"), ~One)), |x| x + 1),
                        (Program::new(~"x", ~If0(~Ident(~"x"), ~One, ~Zero)),
                         |x| if x == 0 {1} else {0}),
                        (Program::new(~"x", ~If0(~Zero, ~One, ~Zero)), |_| 1),
                        (Program::new(~"x", ~If0(~One, ~One, ~Zero)), |_| 0),
                        (Program::new(~"x", fold_expr), fold_fn),
                        ];

        for (p, f) in progs_fn.consume_iter() {
            info!(p.to_str());

            // some "special cases", maybe.
            assert_eq!(p.eval(0), f(0));
            assert_eq!(p.eval(1), f(1));
            assert_eq!(p.eval(-1), f(-1));

            for _ in range(0, 100) {
                let i: u64 = rand::random();
                assert_eq!(p.eval(i), f(i));
            }
        }
    }
}

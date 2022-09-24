use eyre::{bail, Result};
use gg_expr::diagnostic::{Severity, SourceComponent};
use gg_expr::{compile_text, ExtFunc, Map, Vm};
use rustyline::error::ReadlineError;
use rustyline::Editor;

fn main() -> Result<()> {
    let mut editor = Editor::<()>::new()?;

    let mut ctx = Context::new();

    loop {
        let readline = editor.readline(">>> ");
        match readline {
            Ok(line) => {
                ctx.handle_line(&line);
                editor.add_history_entry(&line);
            }
            Err(ReadlineError::Interrupted) => {
                continue;
            }
            Err(ReadlineError::Eof) => {
                break;
            }
            Err(err) => {
                bail!(err);
            }
        }
    }

    Ok(())
}

struct Context {
    env: Map,
    show_bytecode: bool,
    show_time: bool,
}

impl Context {
    fn new() -> Context {
        let mut env = Map::new();

        let mut math = Map::new();

        math.insert("pi".into(), std::f32::consts::PI.into());

        math.insert(
            "sin".into(),
            ExtFunc::new(|ctx, [x]| {
                let x = match x.as_float() {
                    Ok(v) => v,
                    Err(e) => {
                        let ranges = ctx.cur_ranges();
                        let call_range = ranges.as_ref().and_then(|v| v.get(0)).copied();
                        let arg_range = ranges.as_ref().and_then(|v| v.get(2)).copied();
                        let message = format!("{}", e);
                        return Err(ctx.error(call_range, message, |diag, source| {
                            if let (Some(source), Some(range)) = (source, arg_range) {
                                diag.add_source(SourceComponent::new(source).with_label(
                                    Severity::Error,
                                    range,
                                    format!("{:?}", e.found),
                                ));
                            }
                        }));
                    }
                };

                Ok(x.sin().into())
            })
            .into(),
        );

        env.insert("math".into(), math.into());

        Context {
            env,
            show_bytecode: false,
            show_time: false,
        }
    }

    fn handle_line(&mut self, input: &str) {
        if input.trim() == "/b" {
            self.show_bytecode ^= true;
            return;
        }

        if input.trim() == "/t" {
            self.show_time ^= true;
            return;
        }

        let (value, diagnostics) = compile_text(self.env.clone(), &input);

        for diagnostic in &diagnostics {
            println!("{}", diagnostic);
        }

        if !diagnostics.is_empty() {
            return;
        }

        let func = match value {
            Some(v) => v.try_into().unwrap(),
            None => return,
        };

        if self.show_bytecode {
            println!("{:?}", func);
            println!();
        }

        let mut vm = Vm::new();
        let t = std::time::Instant::now();

        match vm.eval(&func, &[]) {
            Ok(v) => println!("{:?}", v),
            Err(e) => {
                eprintln!("{}", e);
            }
        }

        let elapsed = t.elapsed();

        if self.show_time {
            println!("elapsed {:?}", elapsed);
        }
    }
}

use eyre::{bail, Result};
use gg_expr::builtins::builtins;
use gg_expr::{compile_text, Map, Vm};
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
        Context {
            env: builtins(),
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

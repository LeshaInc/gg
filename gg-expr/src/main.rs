use std::io::Read;
use std::sync::Arc;
use std::time::Instant;

use gg_expr::diagnostic::{Component, Diagnostic, Label, Severity, SourceComponent};
use gg_expr::syntax::Parser;
use gg_expr::{compile, Func, Source};

fn main() {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();

    let source = Arc::new(Source::new("unknown.expr".into(), input));
    let mut parser = Parser::new(source.clone());

    let expr = parser.expr();

    println!("{}", expr);

    for diagnostic in parser.diagnostics() {
        println!("{}", diagnostic);
    }

    let (value, diagnostics) = compile(source, &expr);

    for diagnostic in diagnostics {
        println!("{}", diagnostic);
    }

    // if let Ok(thunk) = value.as_thunk() {
    //     let func = thunk.func.as_func().unwrap();
    //     show_spans(func);
    // }

    println!();
    println!("{:?}", value);
    println!();

    let t = Instant::now();
    if let Err(e) = value.force_eval() {
        println!("{}", e);
        return;
    }

    println!("{:?}", value);
    println!("took {:?}", t.elapsed());
}

#[allow(dead_code)]
fn show_spans(func: &Func) {
    let di = match &func.debug_info {
        Some(v) => v,
        _ => return,
    };

    let mut components = Vec::new();

    for (i, spans) in di.instruction_spans.iter().enumerate() {
        if spans.is_empty() {
            continue;
        }

        components.push(Component::Source(SourceComponent {
            source: di.source.clone(),
            labels: spans
                .iter()
                .enumerate()
                .map(|(j, &span)| Label {
                    severity: Severity::Info,
                    span,
                    message: if j == 0 {
                        format!(
                            "{:?} in {}",
                            func.instructions[i],
                            di.source.span_to_line_col(span)
                        )
                    } else {
                        format!("{}", di.source.span_to_line_col(span))
                    },
                })
                .collect(),
        }));
    }

    let diagnostic = Diagnostic {
        severity: Severity::Info,
        message: format!(
            "func {} in {}",
            di.name.as_deref().unwrap_or("?"),
            di.source.span_to_line_col(di.span)
        ),
        components,
    };

    println!("{}", diagnostic);

    for val in func.consts.iter() {
        if let Ok(func) = val.as_func() {
            show_spans(func);
        }
    }
}

# Syntax

In Expr, almost everything is an expression.

This is a simple mathematical expression:

     2 + 2   // ==> 4

Expr supports common aritmhetic operators, such as `+`, `-`, `*`, `/`, `%`
(reminder) and `**` (exponent).

To define a variable, use the `let` expression

    let x = 2 in x * 3    // ==> 6

Multiple variables can be defined in a single expression.
Previously defined variables can be used to compute next variables.

    let x = 2, y = x + 1 in x ** y    // ==> 8 



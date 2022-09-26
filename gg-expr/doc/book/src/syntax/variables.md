# Variables

In Expr, all variables are local and immutable.

To define a variable, use the `let` statement:

    let x = 2 in x * 3    // ==> 6

Multiple variables can be defined in a single expression.
Previously defined variables can be used to compute new variables.

    let x = 2, y = x + 1 in x ** y    // ==> 8 

Variables cannot be defined in terms of themselves, in other words, recursive
values are prohibited. The following is illegal:

    let x = [1, 2, x]  // Error

In other languages, such value would look like this:

    [1, 2, [1, 2, [1, 2, [1, 2, ...]]]]

Mutability, and/or special syntax such as `let rec` allows creation of recursive
values. This implies existance of reference cycles, which are difficult to
implement without a garbage collector or special syntax for weak references.
Expr strives to be simple and performant, so cycles are forbidden.

Note that it doesn't apply to recursive functions. References to functions can
be taken from the call stack, so no cyclic references are needed.
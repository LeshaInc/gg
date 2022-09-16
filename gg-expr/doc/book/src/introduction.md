# Introduction

Expr is a dynamically typed purely functional language inspired by Nix, Clojure,
and Lua.

It is meant to be used as an embedded configuration language in GUI frameworks,
game engines, web servers, etc.

Expr is immutable and doesn't use a (tracing) garbage collector, so it is well
suited for runtime applications, where unpredictable delays are unacceptable.
Instead of a GC, Expr relies on atomic reference counting (ARC). Cyclic
references are explicitly disallowed, though this may change in the future.

Here is a sample of Expr, demonstraiting its main features.

    let
      foo = fn(x, y):
        if x > 0 then
          x * y + x
        else
          y ** 2,
      x = 1,
      y = 2   // this is a comment
    in
      {
        x, y,
        z = foo(x, y),
        list = [1, 0.2e1, "три"]
      }

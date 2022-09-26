# Data Types

## Null

Lack of value. Defined using a `null` literal.

    null 

## Integer

32 bit signed integer. Defined using an integer literal:

    12345 

Support basic operations, such as `+`, `-`, `*`, `/`, `%` (reminder), `**`
(exponentiation). Operation results are promoted to floats, when they go out of
integer range (-2147483648 to 2147483647).

Used for array indexing.

## Float

32 bit IEEE 754 float. Defined using a floating point literal:

    1.0
    876.0e-3
    123e3
    
Support all arithmetic operations, just like integers.

Cannot be used as an array index.

## Bool

Logic type. Defined using `true` and `false` keywords.

Support logic operations, such as `&&` (conjunction), `||` (disjunction), `!`
(logic negation).

    let TO_BE = false
    in  TO_BE || !TO_BE  // ==> true

## String

UTF-8 encoded text. Written in double quotes.

    "hello world"

Literal double quotes can be written inside the string using an escape syntax:
`\"`. Other escape sequences are `\n` (newline), `\r` (carriage return), `\t`
(tab).

Support concatenation using the `+` operator, and repetition using `*`.

    "hello" + " world"  // ==> "hello world"
    "a" * 10  // ==> "aaaaaaaaaa"

## List

List of values. Internally represented using efficient trees, so most operations
take either O(1) or O(log n) time.

    [1, 2, 3]
    ["a", 2, true]

Can be concatenated using `+`, repeated using `*`, and indexed using `arr[idx]`
or `arr?[idx]` (nullable indexing) syntax.

Lists are zero-indexed.

    let list = [1, 2, 3]
    in  list[2]   // ==> 3
    
    [1, 2] * 3  // ==> [1, 2, 1, 2, 1, 2]

## Map

Maps are associative arrays, mapping every key to a value. Keys can be arbitrary
values.

    { x = 1, y = 2 }
    
    let x = 1 in { x }  // same as {x = x}

Maps can be indexed in various ways:

 - `map["key"]` — fallible indexing with an arbitrary key. Here `"key"` can be
   anything, even another map. If the key doesn't exist, an error will be thrown.
 - `map?["key"]` — nullable version of the above. If the key doesn't exist,
   the result is `null`.
 - `map.key` — indexing with an identifier key. Here `key` must be a valid
   identifier, meaning it should consist of letters (`a-z`, `A-Z`), numbers (`0-
   9`), underscore (`_`), and not start from a number.
 - `map?key` — nullable version of the above.

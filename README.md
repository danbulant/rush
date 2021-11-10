# Rush

Rust shell. Inspired by Ion.

In case you're reading this: rush is in the works and not a priority. Features may be missing even if defined below.

## Syntax

`;` is 'alias' for new line.

Syntax and type errors crash the program.

Scopes are for each block. Functions won't have access to variables it wouldn't have access if it wasn't a function:

```sh
fn testing
    echo $t
end

if true
    testing # Error! t is not defined
end
```

### Variables

String variables using `$`, arrays using `@`.

Assigned using `let`.
Left side is evaluated to a string as well.
Variable names must be valid ascii characters, or part of namespace (`namespace::var` syntax).

```sh
let a = d
let $b = c
echo $d # c
```

Arrays are assigned using `[ var ]`. You can join arrays and strings by simply passing them there, like `[ $var @var ]`.

All assignments are done via the `let` keyword.
Instead of `=`, other operations are supported:

* `*=` - multiply
* `+=` - addition
* `-=` - substraction
* `/=` - division
* `//=` - int division
* `%=` - modulo
* `::=` - append
* `@@=` - prepend

#### Special variables

`env::` namespace contains the environment (and doesn't error out if the variable doesn't exist, instead, empty string is returned)
`color::` (alias `c::`) has a number of colors

#### Types

Based on the value set in the set `let` (the one with just `=`), a type is infered (unless specificaly set using `let x:type = ...`). This type is then used for the operations after.

Supported types:

* `i32` (alias int)
* `i64`
* `i128`
* `u32`
* `u64`
* `u128`
* `f32`
* `f64`
* `str`
* `hmap[T]` (where T is one of the other types, except array)
* `[T]` (where T is one of the other types, except hmap)

HashMap is basically array, but with string keys (instead of numbers) in random order.

### Return

Sets the exit code (and possibly exits function/script early). If no return is set, the return code is set to the return code of the last expression.

### Math

Using `$(())` syntax. Math priority applies. Brackets are supported. Variables used without their prepender. Indexing is possible.

### String and Array methods

* $trim(str) - Trims the string (both sides)
* $trimLeft(str) - Trims the string (start/left)
* $trimright(str) - Trims the string (end/right)
* $escape(str) - Escapes string
* $unescape(str) - Unescapes string
* @split(str by) - Splits `str` by `by` into array
* @join(str by=" ") - Joins `str` together `by`

Slices: `[x..y]` gets a substring (or subarray) of the variable. When `x` ommited, defaults to `0` (the start). When `y` ommited, defaults to end.

### Control

Bracketless. Scopes are ended by the keyword `end`.

`if` - Runs it's scope if the command returns `0`. Useful in pair with `test` builtin.
`for $val of @arr` - Runs for each value of the array (or hashmap)
`for $val of ...` - Runs for each number in the range.
`while` - Runs in loop as long as the command returns `0`

### Functions

Defined by `fn name arg -- desc`. `arg` can be ommited, or repeated. `desc` will be printed when the `arg` is missing (or when `describe` command is used).

#### Special

From config (defined by `~/.rushrc`), special functions can be defined.

* `PROMPT` will be run to render the prompt
* `HIGHLIGHT` will run (for each key - make it fast) to highlight the text.

#### Builtins

* `let` for assigning variables
* `export` for exporting variables to env
* `test` tests for evaluation (`=` for equality, `>`, `<`, `<=`, `>=` for number comparisons)
* `exists` for existance of a given string, or if given a flag (`-F`unctions, `-v`ariables, `-e`nv, `-f`ile, `-d`irectory, `-r`eadable file, `-w`ritable file, e`-x`ecutable file), existence of the selected object
* `true` returns `0`
* `false` returns `1`

Some GNU standard utils may be overwritten by rush builtins, but must be made compatible.

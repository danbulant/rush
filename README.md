# Rush

Rust shell. Inspired by Ion.

In case you're reading this: rush is in the works and not a priority. Features may be missing even if defined below.

## Scopes

Variables are block scoped.

Block scope creation:
- `if`
- `while`
- `else`
- `for..`
- `$(expr)`

Functions have a copy of their scope.

Files create file scopes, to which functions are scoped.

## Syntax

`;` is 'alias' for new line.

Syntax and type errors crash the program.

Variables are scoped to their block, and immediately freed when their block is left.

### Variables

String variable value can be obtained using `$`, arrays using `@`.

When an array is stringified (referred to with `$`), it's contents are joined with space.
No special treatment of `PATH`.

Currently, the shell doesn't error out when variable doesn't exist, instead, it's replaced by an empty string.

Assigned using `let`.
Left side is evaluated to a string as well.
Variable names must be valid ascii characters, or part of namespace (`namespace::var` syntax).

```sh
let a = d
let $b = c
echo $d # c
```

Arrays are assigned using `[ var ]`. You can join arrays and strings by simply passing them there, like `[ $var @var ]`.
Arrays and maps cannot be nested during definition (`[ $var [@var] ]` should have the same effect).

All assignments are done via the `let` keyword. If the variable exists, it is overwritten (even in upper scopes).
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

### Return

Sets the exit code (and possibly exits function/script early). If no return is set, the return code is set to the return code of the last expression (`$?`).

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

- `if` - Runs it's scope if the command returns `0`. Useful in pair with `test` builtin. `else` supported. `else if` doesn't require another `end`.
- `for val of @arr` - Runs for each value of the array (or hashmap)
- `for val of X..Y` - Runs for each number in the range `X` and `Y` (both inclusive).
- `while` - Runs in loop as long as the command returns `0`

### Functions

Defined by `fn name [...arg] [--flags]`. `arg` can be ommited, or repeated.
`--flags` can be used to add additional functionality.

Functions are scoped per file, even if they use `on-event` or similar to be triggered.
Use `source` to load external files with functions to be triggered.

- `--desc` sets the functions description.
- `--on-event` will run the function when an event is run

#### Builtins

`let` is a special case which cannot be dynamically addressed (i.e. using `$(echo let) var = value`).

* `let` for assigning variables (`let var = value`)
* `export` for exporting variables to env (`export var` to export var, or `export var = value`)
* `test` tests for evaluation (`=` for equality, `>`, `<`, `<=`, `>=` for number comparisons)
* `exists` for existence of a given string, or if given a flag (`-F`unctions, `-v`ariables, `-e`nv, `-f`ile, `-d`irectory, `-r`eadable file, `-w`ritable file, e`-x`ecutable file), existence of the selected object
* `true` returns `0`
* `false` returns `1`
* `source` to run another file in the same file scope
* `typeof` returns the type of arguments passed
* `inspect` shows the object in a debug output

Some GNU standard utils may be overwritten by rush builtins, but must be made compatible.

# Rush

In case you're reading this: rush is in the works and not a priority. Features may be missing even if defined below.

## Ideating

### Syntax

```sh
set thing = var
set thing = (echo test)
echo test | cowsay
parse file.csv
parse file.csv | sort 2 asc
cat file.csv | parse --csv
parse <(cat file.csv)
<file.csv | parse
parse < file.csv
parse > file.csv
parse < file.csv > file2.csv
echo test >> file

# what bindings do we support here?
# array/object destructors?
# for [i, val] in (cat file.csv | enumerate) might be nice syntax to get line numbers
for i in (parse file.csv) {
    echo $i[1]
}

if (true) {
    echo $i[1]
}

if 1 {}
else {
    echo unused
}

while true {
    break
}

loop {}

fn test (arg) {
    echo $arg
}

# do we error if array literal is used directly in exec call?
# as that would likely be a mistake like $i [property]
echo $i[property] $i[$dynamicproperty]

# or another option - array constructor with a different syntax
# like @[ ] [[ ]]
set array = [var]
# question here, do we allow multiline values? How? \ ?
# or the more classic comma `,` for separating values and ignoring white space?
# this is easier to write so might be preferred for very short scripting lang
set obj = ${
    key: value
    $dynkey: $value2
}
set literal = "$var"
# or perhaps ` ?
set formatted = f"$var"

# these are builtin commands rather than syntax structures (unlike set/while etc)
# they simply accept arguments and work with them as with any other
# builtin commands accept structures rather than strings
test 1 = 1
# perhaps (( x )) could be used for math expressions?
# basically just alias to (calc x)
calc 1 + 1
```

### Values

- String
- Number (f64)
- Object
- Array
    - Array streams
- Void

#### Objects

HashMaps mapping strings to values

#### Array

Arrays mapping integers (0 indexed) to values

#### Void

Acts as undefined for array and object properties that don't exist.

# Instance Methods

Currently there are methods for interacting with types, but they are defined as functions. So for example to push an item to an array:

```
var array = [1, 2, 3];

push(array, 4);

out(fmt(array)); # [1, 2, 3, 4]
```

This was done because it was simpler to code as we already had parsing for functions, they are part of the builtin set of functions, however this is a bit clunky. A more modern way of interacting with data would be to have instance methods:

```
var array = [1, 2, 3];

array.push(4);

out(fmt(array)); # [1, 2, 3, 4]
```

Instance methods should able to be called on any expression.

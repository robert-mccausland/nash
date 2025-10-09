# Guide to writing scripts using Nash

Nash looks quite similar to modern programming languages, rather than a shell language.

## Getting started

Okay so lets start by writing a "hello world" program, in the simplest way:

hello.nash

```nash
#!/bin/nash

"hello, world!";
```

When running it will output the following:

```sh
$ nash hello.nash
hello, world!
```

The first line `#!/bin/nash` is really just a marker to tell the operating system that this is a nash file, its not technically needed - but all the examples here will include it and I would advice always including it. Its similar to how all nash scripts should end with the extension `.nash`, technically its not needed but it helps.

This example might even be a bit _too_ simple to really get what is going on something important to know is that any **statement** that is not assigning to a variable will output its result to `stdout`. Here the statement is a simple string literal `"hello, world!"`, which is terminated with a semicolon, because all statements need to be end with a semicolon.

It might be useful to know how to stop this behavior, which can be done by assigning the output to a variable, the following code will not output anything as there is no bare **statements** which would output something.

hello-2.nash

```nash
#!/bin/nash

var my_variable = "hello, world!";
```

```sh
$ nash hello-2.nash
```

While we are using **variables** we might as well use them a bit, we can use a variable in place of any **literal** value. So the following will effectively be the same as the first example, we are just assigning to a **variable** in between outputing the result.

hello-3.nash

```nash
#!/bin/nash

var my_variable = "hello, world!";
my_variable;
```

```sh
$ nash hello-2.nash
hello, world!
```

Probably should explain **literals** a little bit more, we have already seen the string literal, which is the value you want enclosed by double-quotes. Additionally there are **boolean** and **integer** **literals**.

literals.nash

```nash
#!/bin/nash

var my_int = 123;
var my_boolean = true;
my_int;
my_boolean;
```

```sh
$ nash literals.nash
123
true
```

Okay on to something a bit more interesting, commands. Nash has first class support for spinning up child processes using command, in-fact its really the primary use case. A simple process is shown below:

command.nash

```nash
#!/bin/nash

var (output, error) = exec `echo test`;
output;
```

```sh
$ nash command.nash
test
```

The left hand side stuff looks a bit weird, and we will get to that in a second (its a tuple assignment if you must know). Anything between backticks (\`) is a command, so putting something like \`echo test\` represents the command `echo test`. The first part will be the command, and anything afterwards will be arguments for that command. So the above runs the `echo` command with one argument of `test`.

Because commands can output to `stdout` and `stderr` the result from them comes as a tuple, which is a collection of values. Executed commands will return a tuple with 2 elements, the first being the `stdout` and the second being the `stderr`. In order to extract these into individual variables you can assign like in the example above.

The `exec` keyword is also important, as otherwise the command wouldn't execute, it would just exist as a literal command, not doing anything. This can be helpful because it lets us store the concept of a command, and not just the value that it returned. Also when **piping** commands together its very helpful.

command-1.nash

```nash
#!/bin/nash

`echo test`;
```

```sh
$ nash command-1.nash
echo "test"
```

Piping commands, if you want to pass the output of one command to another you can pipe them together using `=>`. Notice that each command needs it own backticks, you could also defined you commands first and then pipe them. Also by not destructing the tuple we get them output directly to stdout.

command-2.nash

```nash
#!/bin/nash

exec `echo test1` => `grep test`;

var echo = `echo test2`;
exec echo => `grep test`;
```

```sh
$ nash command-2.nash
(test1,)
(test2,)
```

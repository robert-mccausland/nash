# Nash reference documentation

All the gory details about how the language is supposed to work.

## Constructs

- Statements
- Expressions
- Functions
- Blocks
- Root
- Operator
- Literal
- Value

### Value

A **value** is a piece of data, at a very basic level nash scripts are really about doing things with **values**. Almost all the code you write will be about performing logic, but **values** are what that logic is performed on.

There are various types of values in **nash**, all of which represent something slightly different.

There are 5 types in total currently, which include the following basic types:

- `void`
- `string`
- `integer`
- `boolean`

The meaning of all these types should be fairly self descriptive, but more detail on each one should be included somewhere in this guide lol.

In additional there is also the following compound type:

- `tuple`

Compound types contain other values in them, so the `tuple` for example is made up of a collection of other values. It should make a bit more sense once you see it in action.

### Expression

Expressions are really the building blocks of nash, there are a few higher level concepts that sit on top of these, but nearly all of the logic inside a nash program will be contained in an expression.

There are lots of different types of expressions, and the thing that they all have in common is that they will return a **value** when executed.

####

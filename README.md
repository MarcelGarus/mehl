# Mehl

Mehl is a toy programming language that attempts to improve over LISP-like languages.

## Functions have one input and one output

If you view functions as things that can take multiple inputs and then create a single output, you'll naturally arrive at a tree of operations like LISP.
But then you have the function being executed last at the top of your program, which seems weird to me.

If you view functions as things that consume some data and produce some data, you'll naturally arrive at a stack-based programming language.
But then you'll need to know exactly how many things a function consumes and how many it produces to make sense of your program.

Mehl views functions as things that can take exactly *one* input and produces exactly one output.
It feels natural to build chains of functions.
Multiple inputs or outputs can only be modled using tuples.

## Maps are a fundamental thing

## Extensibility is achieved by dynamicness

## For most values, immutability feels natural

## Actors are a fundamental thing of life

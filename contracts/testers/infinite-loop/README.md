# grug-tester-infinite-loop

This contract perform various kinds of infinite loops, to test whether our VM can correctly handle them:

- An infinite loop inside a single `execute` call
- An infinite loop inside a single `query` call
- An `execute` method that calls another `execute` methods that calls another `execute` method...
- A `query` method that calls another `query` method that calls another `query` method...

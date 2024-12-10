# TLA+ generated files

This folder contains TLA+ generated files used to run TLC over the model. See the [Model Checking section here](../../docs/simulation_and_model_checking.md#model-checking) for context.

The files were generated with the command:

``` sh
$ quint compile --target=tlaplus apply_state_machine.qnt  --step=step_fancy --invariant=allInvariants > apply_state_machine.tla
```

but not before making some adjustments to the Quint spec to handle some integration issues. Some adjustments are in the process of being incorporated into the integrated tools (Apalache and Quint), and others that don't have fixes yet, along with some optimizations for model checking, can be found in a `.patch` file in the respective folders.

After generating the TLA+ file, we also manually fixed the `init` definitions, as in TLA+ they can't have the prime operator. This means replacing `tree' :=` with `tree =` for all variables and only inside the `init` defintion.

## Using TLC to model check

Dependencies:
- Install [TLC](https://github.com/tlaplus/tlaplus) (requires Java)
- Install [Quint](https://quint-lang.org/docs/getting-started)

You'll need to run some instance of `quint verify` as that command installs apalache for you, which you'll need for the command below. Go to the folder above this one with all the quint files and run (PS: this command will fail, it is fine):

``` sh
$ quint verify apply_state_machine.qnt --step=step_fancy --max-steps=0
```

Now, Quint should have downloaded Apalache into `~/.quint/`. Check if it's there:

``` sh
ls ~/.quint
```

We can now run TLC. `cd` into either [setupA](./setupA) or [setupB](./setupB) and execute:

``` sh
java -Xmx8G -Xss515m -cp ~/.quint/apalache-dist-0.46.1/apalache/lib/apalache.jar tlc2.TLC -deadlock -workers auto apply_state_machine.tla
```

PS: If you are reading this in the future, you might need to replace `apalache-dist-0.46.1` with a newer version. Use the latest one you have (from the `ls` result above).

This will use up to 8Gb memory for the JVM and automatically select the number of cores (workers) based on the machine it's running on.

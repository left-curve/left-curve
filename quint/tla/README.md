# TLA+ generated files

This folder contains TLA+ generated files used to run TLC over the model.

The files were generated with the command:

``` sh
$ quint compile --target=tlaplus apply_state_machine.qnt  --step=step_fancy --invariant=allInvariants > apply_state_machine.tla
```

but not before making some adjustments to the Quint spec to handle some integration issues. Some adjustments are in the process of being incorporated into the integrated tools (Apalache and Quint), and others that don't have fixes yet, along with some optimizations for model checking, can be found in a `.patch` file in the respective folders.

After generating the TLA+ file, we also manually fixed the `init` definitions, as in TLA+ they can't have the prime operator. This means replacing `tree' :=` with `tree =` for all variables and only inside the `init` defintion.



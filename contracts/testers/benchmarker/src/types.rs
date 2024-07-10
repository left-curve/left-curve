use grug::grug_derive;

#[grug_derive(serde)]
pub enum QueryMsg {
    /// Run a loop of the given number of iterations. Within each iteration, a
    /// set of math operations (addition, subtraction, multiplication, division)
    /// are performed.
    ///
    /// This is used for deducing the relation between Wasmer gas metering
    /// points and CPU time (i.e. how many gas points roughly correspond to one
    /// second of run time).
    Loop { iterations: u64 },
}

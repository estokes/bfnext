# Experimental DCS Mission Scripting in Rust

This is a minimal binding to the DCS lua api. With it, mission scripting can be done almost entirely in Rust. To use it, you build your rust library as a cdylib and load the dll using lua into one or both of the DCS lua environments using the normal lua `require` statement (see the mlua docs about module mode for more details on that). When building you should make sure to instruct mlua to link to the same lua dll that DCS is linking (for obvious reasons), you can do that by setting some environment variables.

It's intended to be as close to a direct translation of the api as possible, while adding safety features only possible with rust, as such it does not force you into IPC, or async, you can choose what works best for your project.

At this time, this api is VERY experimental, expect it to evolve and break a lot.

No one should have to maintain a large project in lua unless they want to :-) civil engineers don't build bridges out of glue, and neither should you.

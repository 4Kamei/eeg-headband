#![cfg_attr(feature = "no_std", no_std)]

capnp::generated_code!(pub mod to_edge, "proto/to_edge_capnp.rs");
capnp::generated_code!(pub mod from_edge, "proto/from_edge_capnp.rs");

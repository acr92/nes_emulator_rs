# NES Emulator in Rust

![Rust Build](https://github.com/acr92/nes_emulator_rs/actions/workflows/rust.yml/badge.svg)

This branch is abandoned. It was an attempt to integrate one-lone coders PPU implementation with that of the NES Emulator in Rust book series implementation. Unfortunately, I couldn't get that to work. This branch is just kept for historical reasons. In case I pick it up in the future.

It works for Donkey Kong and Pac Man, but in Super Mario Bros I only get a blue background, it doesn't render any tiles.

It could be because of compatibility issues between the way one-lone coder implemented their CPU/Bus handling, and how it was done in the NES Emulator in Rust book series. I don't know.

As my objective was to learn Rust, not fight with intricacies of the NES, I'm leaving this branch. I think I can hack something together on the main branch instead.

# Silius Book

_Documentation for Silius users and developers._

[![Telegram Group](https://img.shields.io/endpoint?color=neon&style=flat-square&url=https%3A%2F%2Ftg.sumanjay.workers.dev%2F%2BsKeRcN4j3MM3NmNk)](https://t.me/+sKeRcN4j3MM3NmNk)

_Account Abstraction has for a long time been a dream of the Ethereum developer community._

<i>__- Vitalik Buterin__</i>

__Silius__ is an <a href="https://eips.ethereum.org/EIPS/eip-4337">ERC-4337</a> (account abstraction) bundler implementation that is __modular and highly efficient, written in the memory sasfe programming language Rust.__

<p align="center"><img src="https://raw.githubusercontent.com/silius-rs/silius/main/assets/logo.png" style="border-radius: 20px" width="350px"></p>

## What are the design goals?

**1. Modular**

Architecture of the Silius is composed of multiple components - bundling component, user operation mempool, and JSON-RPC server. This allows you to customize your deployment and run only what you need.

**2. Reusable**

Each major feature is implemented as a separate Rust crate, which makes it possible to reuse code in your project as a building block.

**3. Efficient**

Efficiency is in our mind when we develop and merge new code. 🦀

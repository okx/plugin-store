# uniswap-cca-deployer — Skill Summary

## Overview
The uniswap-cca-deployer skill guides developers through deploying Continuous Clearing Auction (CCA) smart contracts using the Uniswap CCA Factory with CREATE2, which produces deterministic contract addresses that are consistent and reproducible across EVM-compatible chains. It covers factory method invocation, salt selection strategy, deployment transaction construction using Foundry forge scripts, and post-deployment address verification. The skill is part of the `uniswap-cca` plugin and is designed to be used after completing CCA parameter configuration with the uniswap-cca-configurator skill.

## Usage
Install via `claude plugin add @uniswap/uniswap-cca` or `npx skills add Uniswap/uniswap-ai`. Configure parameters with uniswap-cca-configurator first, then use this skill to generate and run the forge deployment script. Confirm deployment details before execution.

## Commands
| Tool | Description |
|---|---|
| `forge script` | Run the CCA factory deployment script |
| `forge verify-contract` | Verify deployed contract on block explorer |

## Triggers
Activates when the user wants to deploy a Continuous Clearing Auction contract, use CREATE2 factory deployment, run a CCA deployer forge script, or asks about CCA contract addresses and deployment verification.

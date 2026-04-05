# uniswap-cca-configurator — Skill Summary

## Overview
The uniswap-cca-configurator skill guides developers through setting up the parameters for a Continuous Clearing Auction (CCA) smart contract on Uniswap V4. CCAs provide a fair and transparent mechanism for token distribution by clearing all bids at a single uniform price. The skill covers auction duration, starting price, minimum clearing price, allocation caps, and other configurable parameters, with AI-assisted validation to catch common misconfiguration issues before deployment. It is part of the `uniswap-cca` plugin and is intended to be used before running the uniswap-cca-deployer.

## Usage
Install via `claude plugin add @uniswap/uniswap-cca` or `npx skills add Uniswap/uniswap-ai`. Use this skill first to determine and validate CCA parameters, then proceed to uniswap-cca-deployer to deploy the configured contracts.

## Commands
This is a reference and configuration skill with no standalone CLI commands. Configuration outputs are Solidity constructor arguments or config structs ready to pass to the CCA factory deployment script.

## Triggers
Activates when the user mentions Continuous Clearing Auction, CCA configuration, token distribution auction parameters, or Uniswap V4 auction setup.

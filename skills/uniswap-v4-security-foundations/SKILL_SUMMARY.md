# uniswap-v4-security-foundations — Skill Summary

## Overview
The uniswap-v4-security-foundations skill provides a security-first reference for Solidity developers building custom hooks on Uniswap V4. It documents known vulnerability patterns in hook contracts — including reentrancy vectors, improper permission flag usage, and callback manipulation — along with concrete mitigations and Solidity coding best practices tailored to the V4 architecture. The skill also outlines audit requirements and a pre-submission checklist to help developers prepare hooks for production deployment. It is part of the `uniswap-hooks` plugin.

## Usage
Install via `claude plugin add @uniswap/uniswap-hooks`. Use this skill during hook development to review code for common vulnerabilities, generate a security checklist, or understand V4-specific attack surfaces before engaging an external auditor.

## Commands
This is a reference skill with no CLI commands. Security review and checklist generation happen through AI-assisted code analysis using the skill's vulnerability pattern catalog and best-practices guide.

## Triggers
Activates when the user mentions Uniswap V4 hooks, hook security, Solidity hook development, V4 vulnerabilities, hook audits, or asks for a security review of a V4 hook contract.

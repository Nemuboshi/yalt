# Agent Instructions

## General

- Use ASD-STE100 for English text.
- Use short sentences and approved words.
- Keep the code simple.
- Make only the changes that the task requires.
- Do not change code that is not part of the task.
- Do not add code or dependencies without a reason.
- Do not change existing behavior without a requirement.
- Do not make assumptions. Get the necessary data.

## Code Analysis

- Use CodeGraph MCP to find dependencies and call paths.
- Use gopls MCP to find Go symbols, types, and references.
- Read only the necessary files and code sections.
- Do not do the same work again without a reason.

## Go

- Use the Go Skills for code design and review.
- Use the applicable Skills before you change Go code.
- Use architecture Skills for changes to packages and APIs.
- Use correctness Skills for errors, resources, and concurrency.
- Use performance Skills for performance tasks.
- Obey the instructions in each Skill.

## Tools

- Use the applicable MCP tools and Skills.
- Use RTK to decrease command output.
- Do not use two tools for the same task without a reason.
- Do not use subagents for simple tasks.

## Verification

- Do the applicable tests and static checks.
- Do a race test after changes to concurrent code.
- Use benchmarks to measure performance changes.
- Examine the final diff.
- Do not ignore errors.
- Do not change tests to conceal defects.

## Report

- Give a short description of the changes.
- Give the test results.
- Identify errors and tests that you did not do.
- Identify risks that remain.
- Use short, factual sentences.
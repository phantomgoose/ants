# Overview

This is an interactive ant colony simulation in Rust.

Run with `cargo run --release` for best performance.

![Ant simulation demo](demo.gif)

## Controls

Press `Space` to pause/unpause, `R` to restart, `Escape` to quit.

Left click to spawn food (orange cells), right click to spawn impassable terrain (yellow cells).

## Home base

Ants have a home base (purple cells at the center of the screen) from which they begin foraging at the start of the
simulation.

Ants drop off food here and refill home base pheromones when stepping on these cells.

## Pheromones

Ants deposit a track of pheromones as they move around. Food pheromones are orange and home pheromones are purple.

Deposited pheromones on the same cell can "stack."
Ants gradually deplete pheromone stores as they move around and refill pheromones when picking up food or visiting their
home. Pheromones also degrade over time. This ensures that the strongest concentration of pheromones is always near the
food, the nest, and along the trails used
by the ants at the present moment.

## Ant state and navigation

Ants have two states - `LookingForFood` and `CarryingFood`.

As they move around, ants probe the environment ahead of them for pheromones (up to `search_radius`). They move towards
the most intense pheromone they can sense.

While the ant is `LookingForFood`, it looks for food pheromones. When it's `CarryingFood`, it looks for home pheromones.
If the ant cannot sense the pheromones it's interested in within its field of view, it moves around randomly.

Ants cannot cross window borders and terrain.

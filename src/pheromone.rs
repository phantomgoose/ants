use std::collections::{HashMap, HashSet};

use macroquad::math::Rect;
use macroquad::prelude::Color;
use rayon::prelude::*;

use crate::ant::ANT_RANDOM_WALK_MAX_ROTATION;
use crate::grid::{GridLocation, NEST_COLOR, WorldGrid};
use crate::util::{normalize_angle, RectExtensions};

const MAX_FOOD_PHEROMONE_OPACITY: f32 = 0.75;
const MAX_HOME_PHEROMONE_OPACITY: f32 = 0.75;
const PHEROMONE_FOOD_COLOR: Color = Color::new(1.00, 0.65, 0.50, MAX_FOOD_PHEROMONE_OPACITY);
const PHEROMONE_DECAY_RATE: f32 = 0.4;
const PHEROMONE_DETECTION_MINIMUM: f32 = 0.01; // minimum pheromone health at which it is still detectable. Removed from the world below this value.
const PHEROMONE_INTENSITY_MAX: f32 = 1000.;
pub const SPECIAL_PHEROMONE_INTENSITY: f32 = 10000.;

// Directions to check for pheromones. Something like the following:
//   |/
// ant--
//   |\
const PHEROMONE_SEARCH_DIRECTIONS: [f32; 5] = [
    -ANT_RANDOM_WALK_MAX_ROTATION,
    -ANT_RANDOM_WALK_MAX_ROTATION / 2.,
    0.,
    ANT_RANDOM_WALK_MAX_ROTATION / 2.,
    ANT_RANDOM_WALK_MAX_ROTATION,
];

#[derive(Copy, Clone)]
pub enum PheromoneType {
    Food,
    Home,
}

#[derive(Copy, Clone)]
pub struct Pheromone {
    intensity: f32, // diminishes over time
    pheromone_type: PheromoneType,
    rect: Rect,
    decayed: bool,
    locked_intensity: bool,
}

impl Pheromone {
    pub fn new(
        intensity: f32,
        pheromone_type: PheromoneType,
        rect: Rect,
        locked_intensity: bool,
    ) -> Self {
        Self {
            intensity,
            pheromone_type,
            rect,
            decayed: false,
            locked_intensity,
        }
    }
    pub fn draw(&self) {
        // pheromone opacity depends on its intensity level
        let color = match self.pheromone_type {
            PheromoneType::Food => Color {
                a: (self.intensity * MAX_FOOD_PHEROMONE_OPACITY).min(MAX_FOOD_PHEROMONE_OPACITY),
                ..PHEROMONE_FOOD_COLOR
            },
            PheromoneType::Home => Color {
                a: self
                    .intensity
                    .min(MAX_HOME_PHEROMONE_OPACITY)
                    .min(MAX_HOME_PHEROMONE_OPACITY),
                ..NEST_COLOR
            },
        };

        self.rect.draw_rectangle(color);
    }

    pub fn tick(&mut self, dt: f32) {
        if self.locked_intensity || self.decayed {
            // locked pheromones (like those on food cells) don't degrade over time
            return;
        }

        self.intensity *= 1.0 - (dt * PHEROMONE_DECAY_RATE);
        if self.intensity < PHEROMONE_DETECTION_MINIMUM {
            self.decayed = true
        }
    }

    pub fn increase_intensity(&mut self, additional_intensity: f32) {
        if self.locked_intensity {
            return;
        }

        // cap intensity at intensity max
        self.intensity = (self.intensity + additional_intensity).min(PHEROMONE_INTENSITY_MAX);
    }

    pub fn decayed(&self) -> bool {
        self.decayed
    }

    pub fn rect(&self) -> &Rect {
        &self.rect
    }

    pub fn intensity(&self) -> f32 {
        self.intensity
    }

    pub fn pheromone_type(&self) -> &PheromoneType {
        &self.pheromone_type
    }

    pub fn locked_intensity(&self) -> bool {
        self.locked_intensity
    }
}

pub struct Pheromones {
    pub entries: HashMap<GridLocation, Pheromone>,
}

impl Pheromones {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Returns the pheromone that the ant should turn towards, if any
    pub fn get_pheromone_to_target(
        &self,
        grid: &WorldGrid,
        ant_rect: &Rect,
        rotation: f32,
        search_radius: f32,
    ) -> Option<Pheromone> {
        self.get_nearby_pheromones(grid, ant_rect, rotation, search_radius)
            .iter()
            .max_by(|p1, p2| p1.intensity().total_cmp(&p2.intensity()))
            .map(|ph| **ph)
    }

    fn get_nearby_pheromones(
        &self,
        grid: &WorldGrid,
        source_rect: &Rect,
        rotation: f32,
        search_radius: f32,
    ) -> Vec<&Pheromone> {
        let mut results = Vec::new();

        for dir in PHEROMONE_SEARCH_DIRECTIONS {
            if let Some(most_intense_pheromone) = grid
                // get all cells in target direction
                .get_cells_in_direction(source_rect, normalize_angle(rotation + dir), search_radius)
                .iter()
                // get all the pheromones occupying the cells in that direction
                .filter_map(|loc| self.entries.get(loc))
                // keep only the most intense pheromone in that direction
                .max_by(|p1, p2| p1.intensity.total_cmp(&p2.intensity))
            {
                results.push(most_intense_pheromone);
            }
        }

        results
    }

    pub fn tick(&mut self, dt: f32) {
        let expired_pheromone_locs: Vec<GridLocation> = self
            .entries
            .par_iter_mut()
            .fold(HashSet::new, |mut expired_pheromones, (loc, pheromone)| {
                pheromone.tick(dt);
                if pheromone.decayed() {
                    expired_pheromones.insert(*loc);
                }
                expired_pheromones
            })
            .flatten()
            .collect();
        for loc in expired_pheromone_locs {
            self.entries.remove(&loc);
        }
    }
}

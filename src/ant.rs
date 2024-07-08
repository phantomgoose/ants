use std::f32::consts::PI;

use macroquad::color::GREEN;
use macroquad::experimental::animation::{AnimatedSprite, Animation};
use macroquad::math::Vec2;
use macroquad::prelude::{
    Color, draw_line, draw_rectangle, draw_texture_ex, DrawTextureParams, Rect, Texture2D, WHITE,
};
use macroquad::rand::gen_range;
use macroquad::shapes::draw_circle_lines;
use macroquad::text::draw_text;

use crate::DEBUG;
use crate::grid::{CellType, FOOD_COLOR, GRID_WIDTH, GridLocation, WorldGrid};
use crate::pheromone::{Pheromone, PheromoneType};
use crate::util::normalize_angle;

const ANT_ANIMATION_FPS: u32 = 200;
const ANT_SIZE_MULTIPLIER: f32 = 1. / 20.;
const BASE_ANT_MOVE_SPEED: f32 = 100.;
const ANT_SPEED_RANDOM_FACTOR: f32 = 0.3; // how much of the move and rotation speed is randomized
const ANT_BASE_WIDTH: u32 = 202;
const ANT_BASE_HEIGHT: u32 = 248;
const ANT_WIDTH: f32 = ANT_BASE_WIDTH as f32 * ANT_SIZE_MULTIPLIER;
const ANT_HEIGHT: f32 = ANT_BASE_HEIGHT as f32 * ANT_SIZE_MULTIPLIER;
// rotate the ant 90 degrees to account for it facing upwards in the tileset rather than to the right
const ANT_SPRITE_ROTATION_CORRECTION: f32 = PI * 90. / 180.;
const CELLS_WIDTHS_BETWEEN_PHEROMONES: f32 = 0.23;
const ANT_GRID_SENSES_PERCENT: f32 = 0.1; // percentage of the grid's width the ants can sense
const ANT_PHEROMONE_RETAIN_RATIO: f32 = 0.99; // how much of carried pheromone remains after dropping some
const ANT_PHEROMONE_BASE_INTENSITY: f32 = 1.;
const ANT_TIME_BETWEEN_STATE_CHECKS: f32 = 0.1;
pub const ANT_RANDOM_WALK_MAX_ROTATION: f32 = PI / 4.;
const DEFAULT_ANT_COLOR: Color = WHITE;

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum AntState {
    // RandomlySearching,
    CarryingFood,
    LookingForFood,
}

pub enum AntActionTaken {
    PickedUpFood,
    DroppedOffFood,
    HitTerrain,
}

pub struct Ant<'a> {
    tileset: &'a Texture2D,
    animated_sprite: AnimatedSprite,
    animation_count: usize,
    rotation: f32,
    rect: Rect,
    move_speed: f32,
    distance_since_last_pheromone: f32,
    state: AntState,
    pheromone_intensity: f32,
    dt_since_last_update: f32, // how long ago the ant last checked its bearings
    search_radius: f32,
    distance_between_pheromones: f32,
}

fn get_animation_for_idx(idx: u32, frames: u32, fps: u32) -> Animation {
    Animation {
        name: format!("walk{}", idx),
        row: idx,
        frames,
        fps,
    }
}

impl<'a> Ant<'a> {
    pub fn draw(&mut self) {
        let ant_sprite = &mut self.animated_sprite;

        let color = match self.state {
            AntState::CarryingFood => FOOD_COLOR,
            AntState::LookingForFood => DEFAULT_ANT_COLOR,
        };

        draw_texture_ex(
            self.tileset,
            self.rect.x,
            self.rect.y,
            color,
            DrawTextureParams {
                source: Some(ant_sprite.frame().source_rect),
                dest_size: Some(ant_sprite.frame().dest_size * ANT_SIZE_MULTIPLIER),
                rotation: self.rotation + ANT_SPRITE_ROTATION_CORRECTION,
                ..DrawTextureParams::default()
            },
        );

        if DEBUG {
            // search radius
            draw_circle_lines(
                self.rect.center().x,
                self.rect.center().y,
                self.search_radius,
                2.,
                GREEN,
            );

            // ant bounding box
            draw_rectangle(self.rect.x, self.rect.y, self.rect.w, self.rect.h, WHITE);

            // draw direction of the ant
            let direction = Vec2::new(self.rotation.cos(), self.rotation.sin());
            draw_line(
                self.rect.center().x,
                self.rect.center().y,
                self.rect.center().x + direction.x * 20.,
                self.rect.center().y + direction.y * 20.,
                1.,
                GREEN,
            );

            // draw rotation value
            let msg = format!("Rotation: {}", self.rotation);
            draw_text(msg.as_str(), self.rect.x, self.rect.y, 10., WHITE);
        }

        // loop animation
        if ant_sprite.is_last_frame() {
            ant_sprite.set_animation((ant_sprite.current_animation() + 1) % self.animation_count);
            ant_sprite.set_frame(0);
        } else {
            ant_sprite.update();
        }
    }

    pub fn new(x: f32, y: f32, tileset: &'a Texture2D, grid: &WorldGrid) -> Self {
        let frame_counts: [u32; 8] = [8, 8, 8, 8, 8, 8, 8, 6];
        let animated_sprite = AnimatedSprite::new(
            ANT_BASE_WIDTH,
            ANT_BASE_HEIGHT,
            &frame_counts
                .iter()
                .enumerate()
                .map(|(i, frames)| get_animation_for_idx(i as u32, *frames, ANT_ANIMATION_FPS))
                .collect::<Vec<Animation>>(),
            true,
        );

        let distance_between_pheromones = CELLS_WIDTHS_BETWEEN_PHEROMONES * grid.cell_width;

        Ant {
            tileset,
            animated_sprite,
            animation_count: frame_counts.len(),
            rotation: gen_range(-PI, PI),
            move_speed: gen_range(1.0 - ANT_SPEED_RANDOM_FACTOR, 1.0 + ANT_SPEED_RANDOM_FACTOR)
                * BASE_ANT_MOVE_SPEED,
            rect: Rect::new(
                x - (ANT_WIDTH / 2.),
                y - (ANT_HEIGHT / 2.),
                ANT_WIDTH,
                ANT_HEIGHT,
            ),
            distance_since_last_pheromone: 0.,
            state: AntState::LookingForFood,
            pheromone_intensity: ANT_PHEROMONE_BASE_INTENSITY,
            dt_since_last_update: gen_range(0., ANT_TIME_BETWEEN_STATE_CHECKS),
            search_radius: ANT_GRID_SENSES_PERCENT * GRID_WIDTH as f32 * grid.cell_width,
            distance_between_pheromones,
        }
    }

    /// Returns the angle to the target pheromone
    fn get_target_angle(&self, pheromone: Pheromone) -> f32 {
        let direction = (pheromone.rect().center() - self.rect.center()).normalize_or_zero();
        direction.y.atan2(direction.x)
    }

    /// Instantly turns the ant towards the target angle
    fn snap_towards(&mut self, target_angle: f32) {
        self.rotation = normalize_angle(target_angle);
    }

    /// Walks straight given its current rotation and respecting the boundaries of the world
    fn walk_straight(&mut self, bounding_box: &Rect, dt: f32) {
        let direction = Vec2::new(self.rotation.cos(), self.rotation.sin());

        self.rect.x += direction.x * self.move_speed * dt;
        self.rect.y += direction.y * self.move_speed * dt;

        // keep the ant within world boundary
        if self.rect.x < bounding_box.x {
            self.rotation = normalize_angle(PI - self.rotation);
            self.rect.x = bounding_box.x;
        } else if self.rect.x + self.rect.w > bounding_box.w {
            self.rotation = normalize_angle(PI - self.rotation);
            self.rect.x = bounding_box.w - self.rect.w;
        } else if self.rect.y < bounding_box.y {
            self.rotation = normalize_angle(-self.rotation);
            self.rect.y = bounding_box.y;
        } else if self.rect.y + self.rect.h > bounding_box.h {
            self.rotation = normalize_angle(-self.rotation);
            self.rect.y = bounding_box.h - self.rect.h;
        }
    }

    /// Turn in a random new direction to avoid collision
    fn bounce_off(&mut self) {
        // TODO: revisit and refactor
        if rand::random() {
            self.rotation = normalize_angle(-self.rotation);
        } else {
            self.rotation = normalize_angle(PI - self.rotation);
        }
    }

    fn walk_to_pheromones(&mut self, grid: &WorldGrid, dt: f32) {
        // dont change direction too often
        if self.dt_since_last_update < ANT_TIME_BETWEEN_STATE_CHECKS {
            self.dt_since_last_update += dt;
            // dont attempt to change direction too often, likely to cause weird ant behavior
            self.walk_straight(grid.bounding_box(), dt);
            return;
        }

        self.dt_since_last_update = 0.; // reset behavior change timer
        let candidate_pheromones = match self.state {
            AntState::LookingForFood => grid.pheromones(PheromoneType::Food),
            AntState::CarryingFood => grid.pheromones(PheromoneType::Home),
        };

        let target_angle = if let Some(pheromone) = candidate_pheromones.get_pheromone_to_target(
            grid,
            &self.rect,
            self.rotation,
            self.search_radius,
        ) {
            // if we found a pheromone in our field of view, turn towards it
            self.get_target_angle(pheromone)
        } else {
            // otherwise turn randomly
            self.rotation + gen_range(-ANT_RANDOM_WALK_MAX_ROTATION, ANT_RANDOM_WALK_MAX_ROTATION)
        };

        // walk in the direction we picked
        self.snap_towards(target_angle);
        self.walk_straight(grid.bounding_box(), dt);
    }

    pub fn tick(
        &mut self,
        grid: &WorldGrid,
        dt: f32,
    ) -> (GridLocation, Option<Pheromone>, Option<AntActionTaken>) {
        // walk
        let starting_point = self.rect;

        self.walk_to_pheromones(grid, dt);

        let ending_point = self.rect;
        let distance_walked = starting_point
            .center()
            .distance(ending_point.center())
            .abs();
        self.distance_since_last_pheromone += distance_walked;

        let ending_location = grid
            .get_grid_location(ending_point.center().x, ending_point.center().y)
            .expect("Ants should never walk off the world grid.");

        // check for collision with important cells and update ant state
        let mut action_taken = None;
        let prev_state = self.state;
        let current_cell = grid.get_cell_for_loc(ending_location);

        match current_cell.cell_type() {
            CellType::Food(_) => {
                self.state = AntState::CarryingFood;
                self.pheromone_intensity = ANT_PHEROMONE_BASE_INTENSITY;
            }
            CellType::Home => {
                self.state = AntState::LookingForFood;
                self.pheromone_intensity = ANT_PHEROMONE_BASE_INTENSITY;
            }
            CellType::Terrain => {
                self.walk_straight(grid.bounding_box(), -dt); // return to starting position
                self.bounce_off(); // turn in a safer direction
                let loc = grid
                    .get_grid_location_for_rect(&self.rect)
                    .expect("ant should end up in a valid location");
                return (loc, None, Some(AntActionTaken::HitTerrain));
            }
            _ => {}
        }

        if prev_state != self.state {
            action_taken = Some(match self.state {
                AntState::CarryingFood => AntActionTaken::PickedUpFood,
                AntState::LookingForFood => AntActionTaken::DroppedOffFood,
            })
        }

        // spawn pheromone if it's time to do so
        let mut pheromone = None;
        if self.distance_since_last_pheromone >= self.distance_between_pheromones {
            self.distance_since_last_pheromone = 0.;
            let pheromone_type = match self.state {
                AntState::CarryingFood => PheromoneType::Food,
                AntState::LookingForFood => PheromoneType::Home,
            };

            pheromone = Some(grid.create_pheromone_for_loc(
                ending_location,
                pheromone_type,
                self.pheromone_intensity,
                false,
            ));
            self.pheromone_intensity *= ANT_PHEROMONE_RETAIN_RATIO;
        }

        (ending_location, pheromone, action_taken)
    }

    pub fn state(&self) -> AntState {
        self.state
    }
}

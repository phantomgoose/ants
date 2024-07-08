use macroquad::prelude::*;
use rayon::prelude::*;

use crate::ant::{Ant, AntActionTaken};
use crate::grid::{
    CellType, FOOD_CONSUMPTION_LIMIT, GRID_HEIGHT, GRID_WIDTH, GridLocation, WorldGrid,
};
use crate::pheromone::Pheromone;

mod ant;
mod grid;
mod pheromone;
mod util;

const DEBUG: bool = false;
const ANT_COUNT: usize = 1_000;

#[macroquad::main("Ants")]
async fn main() {
    let world_bounding_box = Rect::new(0., 0., screen_width(), screen_height());

    let ant_tileset = load_texture("assets/ant.png").await.unwrap();

    let (mut ants, mut paused, mut grid) = init(&ant_tileset);

    loop {
        let keys_pressed = get_keys_pressed();
        if keys_pressed.contains(&KeyCode::Escape) {
            // quit
            break;
        }

        if keys_pressed.contains(&KeyCode::Space) {
            // pause
            paused = !paused;
        }

        if keys_pressed.contains(&KeyCode::R) {
            // reset
            (ants, paused, grid) = init(&ant_tileset);
        }

        if is_mouse_button_down(MouseButton::Left) {
            let (x, y) = mouse_position();
            grid.spawn_cells(x, y, CellType::Food(FOOD_CONSUMPTION_LIMIT))
        } else if is_mouse_button_down(MouseButton::Right) {
            let (x, y) = mouse_position();
            grid.spawn_cells(x, y, CellType::Terrain)
        }

        if !paused {
            let dt = get_frame_time();

            grid.tick(dt);
            let ant_state_updates: Vec<(GridLocation, Option<Pheromone>, Option<AntActionTaken>)> =
                ants.par_iter_mut().map(|ant| ant.tick(&grid, dt)).collect();

            ant_state_updates.into_iter().for_each(|(loc, ph, action)| {
                // deposit pheromone on the grid if it was spawned by the ant
                if let Some(pheromone) = ph {
                    grid.deposit_pheromone(pheromone)
                }
                grid.visit_cell(loc, action);
            });
        }

        clear_background(BLACK);
        grid.draw(&ants);
        ants.iter_mut().for_each(|ant| ant.draw());

        if DEBUG {
            draw_line(
                world_bounding_box.x,
                world_bounding_box.y,
                world_bounding_box.w,
                world_bounding_box.h,
                1.,
                GREEN,
            );
        }

        next_frame().await
    }
}

fn init(ant_tileset: &Texture2D) -> (Vec<Ant>, bool, WorldGrid) {
    let home_cells: usize = 10;
    let home_start_row: usize = GRID_HEIGHT / 2 - home_cells / 2;
    let home_start_col: usize = GRID_WIDTH / 2 - home_cells / 2;

    let mut home_locs = Vec::new();
    for r in home_start_row..home_start_row + home_cells {
        for c in home_start_col..home_start_col + home_cells {
            home_locs.push(GridLocation::new(r, c));
        }
    }

    let sw = screen_width();
    let sh = screen_height();

    let grid = WorldGrid::new(home_locs.as_slice(), sw, sh);

    let grid_center_loc = GridLocation::new(
        home_start_row + home_cells / 2,
        home_start_col + home_cells / 2,
    );
    let ant_spawn_point = grid.get_rect_from_loc(grid_center_loc);
    let ants = std::iter::repeat_with(|| {
        Ant::new(
            ant_spawn_point.center().x,
            ant_spawn_point.center().y,
            ant_tileset,
            &grid,
        )
    })
    .take(ANT_COUNT)
    .collect::<Vec<Ant>>();

    let paused = false;

    (ants, paused, grid)
}

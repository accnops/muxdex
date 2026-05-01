use ratatui::layout::Rect;

pub fn compute_grid(area: Rect, count: usize) -> Vec<Rect> {
    if count == 0 || area.width == 0 || area.height == 0 {
        return Vec::new();
    }

    let columns = (count as f64).sqrt().ceil() as u16;
    let rows = ((count as u16) + columns - 1) / columns;
    let cell_width = (area.width / columns).max(1);
    let cell_height = (area.height / rows).max(1);
    let mut rects = Vec::with_capacity(count);

    for index in 0..count {
        let row = (index as u16) / columns;
        let column = (index as u16) % columns;
        let x = area.x + (column * cell_width);
        let y = area.y + (row * cell_height);
        let width = if column == columns - 1 {
            area.width.saturating_sub(column * cell_width)
        } else {
            cell_width
        };
        let height = if row == rows - 1 {
            area.height.saturating_sub(row * cell_height)
        } else {
            cell_height
        };

        rects.push(Rect {
            x,
            y,
            width,
            height,
        });
    }

    rects
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::compute_grid;

    #[test]
    fn layout_places_four_tiles_in_two_by_two_grid() {
        let rects = compute_grid(
            Rect {
                x: 0,
                y: 0,
                width: 120,
                height: 40,
            },
            4,
        );

        assert_eq!(rects.len(), 4);
        assert_eq!(rects[0].width, rects[1].width);
        assert_eq!(rects[0].height, rects[2].height);
    }
}

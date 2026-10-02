use std::iter::zip;

use niri_config::{CornerRadius, Gradient, GradientRelativeTo};
use smithay::backend::renderer::element::{Element as _, Kind};
use smithay::utils::{Logical, Point, Rectangle, Size};

use crate::niri_render_elements;
use crate::render_helpers::border::{BorderRenderElement, BorderRenderParams};
use crate::render_helpers::renderer::NiriRenderer;
use crate::render_helpers::solid_color::{SolidColorBuffer, SolidColorRenderElement};

#[derive(Debug)]
pub struct FocusRing {
    buffers: [SolidColorBuffer; 8],
    locations: [Point<f64, Logical>; 8],
    sizes: [Size<f64, Logical>; 8],
    borders: [BorderRenderElement; 8],
    full_size: Size<f64, Logical>,
    is_border: bool,
    use_border_shader: bool,
    config: niri_config::FocusRing,
    thicken_corners: bool,
}

niri_render_elements! {
    FocusRingRenderElement => {
        SolidColor = SolidColorRenderElement,
        Gradient = BorderRenderElement,
    }
}

impl FocusRing {
    pub fn new(config: niri_config::FocusRing) -> Self {
        Self {
            buffers: Default::default(),
            locations: Default::default(),
            sizes: Default::default(),
            borders: Default::default(),
            full_size: Default::default(),
            is_border: false,
            use_border_shader: false,
            config,
            thicken_corners: true,
        }
    }

    pub fn update_config(&mut self, config: niri_config::FocusRing) {
        self.config = config;
    }

    pub fn update_shaders(&mut self) {
        for elem in &mut self.borders {
            elem.damage_all();
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_render_elements(
        &mut self,
        win_size: Size<f64, Logical>,
        is_active: bool,
        is_border: bool,
        is_urgent: bool,
        view_rect: Rectangle<f64, Logical>,
        radius: CornerRadius,
        scale: f64,
        alpha: f32,
        fade: f32,
        gradient_angle_offset: f32,
    ) {
        let width = self.config.width;
        self.full_size = win_size + Size::from((width, width)).upscale(2.);
        self.is_border = is_border;

        // During a fade-out, keep showing active colors until the fade completes; this also
        // matches what the solid-color fallback path renders while the border shader is missing.
        let show_active = is_active || fade > 0.;

        let color = if is_urgent {
            self.config.urgent_color
        } else if show_active {
            self.config.active_color
        } else {
            self.config.inactive_color
        };

        for buf in &mut self.buffers {
            buf.set_color(color);
        }

        let radius = radius.fit_to(self.full_size.w as f32, self.full_size.h as f32);

        // The two color pairs that the shader crossfades between. Urgent wins immediately:
        // both pairs become the urgent colors, neutralizing the fade.
        let active = self
            .config
            .active_gradient
            .unwrap_or_else(|| Gradient::from(self.config.active_color));
        let inactive = self
            .config
            .inactive_gradient
            .unwrap_or_else(|| Gradient::from(self.config.inactive_color));
        let urgent = self
            .config
            .urgent_gradient
            .unwrap_or_else(|| Gradient::from(self.config.urgent_color));
        let (active, inactive) = if is_urgent {
            (urgent, urgent)
        } else {
            (active, inactive)
        };

        // The representative gradient determines the gradient area, interpolation, and base
        // angle for this frame. Steady states pick the corresponding pair exactly; mid-fade
        // the pairs may differ in these properties, which is invisible in practice.
        let gradient = if show_active { active } else { inactive };
        let has_user_gradient = if show_active {
            self.config.active_gradient.is_some()
        } else {
            self.config.inactive_gradient.is_some()
        };

        // Use the border shader for gradients, rounded corners, or an in-progress crossfade.
        // If the shader is missing, the solid-color path renders an instant cut instead.
        self.use_border_shader =
            radius != CornerRadius::default() || has_user_gradient || (fade != 0. && fade != 1.);

        let full_rect = Rectangle::new(Point::from((-width, -width)), self.full_size);
        let gradient_area = match gradient.relative_to {
            GradientRelativeTo::Window => full_rect,
            GradientRelativeTo::WorkspaceView => view_rect,
        };

        let rounded_corner_border_width = if is_border {
            // HACK: increase the border width used for the inner rounded corners a tiny bit to
            // reduce background bleed.
            let extra = if self.thicken_corners { 0.5 } else { 0. };
            width as f32 + extra
        } else {
            0.
        };

        let ceil = |logical: f64| (logical * scale).ceil() / scale;

        // All of this stuff should end up aligned to physical pixels because:
        // * Window size and border width are rounded to physical pixels before being passed to this
        //   function.
        // * We will ceil the corner radii below.
        // * We do not divide anything, only add, subtract and multiply by integers.
        // * At rendering time, tile positions are rounded to physical pixels.

        let angle = ((gradient.angle as f32) - 90.).to_radians() + gradient_angle_offset;

        let params = BorderRenderParams {
            size: Default::default(),
            gradient_area: Rectangle::new(gradient_area.loc, gradient_area.size),
            gradient_format: gradient.in_,
            color_from: active.from,
            color_to: active.to,
            color_from_inactive: inactive.from,
            color_to_inactive: inactive.to,
            fade,
            angle,
            geometry: Default::default(),
            border_width: rounded_corner_border_width,
            corner_radius: radius,
            scale: scale as f32,
            alpha,
        };

        if is_border {
            let top_left = f64::max(width, ceil(f64::from(radius.top_left)));
            let top_right = f64::min(
                self.full_size.w - top_left,
                f64::max(width, ceil(f64::from(radius.top_right))),
            );
            let bottom_left = f64::min(
                self.full_size.h - top_left,
                f64::max(width, ceil(f64::from(radius.bottom_left))),
            );
            let bottom_right = f64::min(
                self.full_size.h - top_right,
                f64::min(
                    self.full_size.w - bottom_left,
                    f64::max(width, ceil(f64::from(radius.bottom_right))),
                ),
            );

            // Top edge.
            self.sizes[0] = Size::from((win_size.w + width * 2. - top_left - top_right, width));
            self.locations[0] = Point::from((-width + top_left, -width));

            // Bottom edge.
            self.sizes[1] =
                Size::from((win_size.w + width * 2. - bottom_left - bottom_right, width));
            self.locations[1] = Point::from((-width + bottom_left, win_size.h));

            // Left edge.
            self.sizes[2] = Size::from((width, win_size.h + width * 2. - top_left - bottom_left));
            self.locations[2] = Point::from((-width, -width + top_left));

            // Right edge.
            self.sizes[3] = Size::from((width, win_size.h + width * 2. - top_right - bottom_right));
            self.locations[3] = Point::from((win_size.w, -width + top_right));

            // Top-left corner.
            self.sizes[4] = Size::from((top_left, top_left));
            self.locations[4] = Point::from((-width, -width));

            // Top-right corner.
            self.sizes[5] = Size::from((top_right, top_right));
            self.locations[5] = Point::from((win_size.w + width - top_right, -width));

            // Bottom-right corner.
            self.sizes[6] = Size::from((bottom_right, bottom_right));
            self.locations[6] = Point::from((
                win_size.w + width - bottom_right,
                win_size.h + width - bottom_right,
            ));

            // Bottom-left corner.
            self.sizes[7] = Size::from((bottom_left, bottom_left));
            self.locations[7] = Point::from((-width, win_size.h + width - bottom_left));

            for (buf, size) in zip(&mut self.buffers, self.sizes) {
                buf.resize(size);
            }

            for (border, (loc, size)) in zip(&mut self.borders, zip(self.locations, self.sizes)) {
                border.update(BorderRenderParams {
                    size,
                    gradient_area: Rectangle::new(gradient_area.loc - loc, gradient_area.size),
                    geometry: Rectangle::new(full_rect.loc - loc, full_rect.size),
                    ..params
                });
            }
        } else {
            self.sizes[0] = self.full_size;
            self.buffers[0].resize(self.sizes[0]);
            self.locations[0] = Point::from((-width, -width));

            let loc = self.locations[0];
            self.borders[0].update(BorderRenderParams {
                size: self.sizes[0],
                gradient_area: Rectangle::new(gradient_area.loc - loc, gradient_area.size),
                geometry: Rectangle::new(full_rect.loc - loc, full_rect.size),
                ..params
            });
        }
    }

    pub fn render(
        &self,
        renderer: &mut impl NiriRenderer,
        location: Point<f64, Logical>,
        push: &mut dyn FnMut(FocusRingRenderElement),
    ) {
        if self.config.off {
            return;
        }

        let border_width = -self.locations[0].y;

        // If drawing as a border with width = 0, then there's nothing to draw.
        if self.is_border && border_width == 0. {
            return;
        }

        let has_border_shader = BorderRenderElement::has_shader(renderer);

        let mut push = |buffer, border: &BorderRenderElement, location: Point<f64, Logical>| {
            let elem = if self.use_border_shader && has_border_shader {
                border.clone().with_location(location).into()
            } else {
                let alpha = border.alpha();
                SolidColorRenderElement::from_buffer(buffer, location, alpha, Kind::Unspecified)
                    .into()
            };
            push(elem);
        };

        if self.is_border {
            for ((buf, border), loc) in zip(zip(&self.buffers, &self.borders), self.locations) {
                push(buf, border, location + loc);
            }
        } else {
            push(
                &self.buffers[0],
                &self.borders[0],
                location + self.locations[0],
            );
        }
    }

    pub fn width(&self) -> f64 {
        self.config.width
    }

    pub fn is_off(&self) -> bool {
        self.config.off
    }

    pub fn set_thicken_corners(&mut self, value: bool) {
        self.thicken_corners = value;
    }

    pub fn config(&self) -> &niri_config::FocusRing {
        &self.config
    }
}

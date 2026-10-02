use niri::render_helpers::border::{BorderRenderElement, BorderRenderParams};
use niri_config::{
    Color, CornerRadius, GradientColorSpace, GradientInterpolation, HueInterpolation,
};
use smithay::backend::renderer::element::RenderElement;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::utils::{Physical, Point, Rectangle, Size};

use super::{Args, TestCase};

pub struct GradientOklchShorter {
    gradient_format: GradientInterpolation,
}

impl GradientOklchShorter {
    pub fn new(_args: Args) -> Self {
        Self {
            gradient_format: GradientInterpolation {
                color_space: GradientColorSpace::Oklch,
                hue_interpolation: HueInterpolation::Shorter,
            },
        }
    }
}

impl TestCase for GradientOklchShorter {
    fn render(
        &mut self,
        _renderer: &mut GlesRenderer,
        size: Size<i32, Physical>,
    ) -> Vec<Box<dyn RenderElement<GlesRenderer>>> {
        let (a, b) = (size.w / 6, size.h / 3);
        let size = (size.w - a * 2, size.h - b * 2);
        let area = Rectangle::new(Point::from((a, b)), Size::from(size)).to_f64();

        [BorderRenderElement::new(BorderRenderParams {
            size: area.size,
            gradient_area: Rectangle::from_size(area.size),
            gradient_format: self.gradient_format,
            color_from: Color::new_unpremul(1., 0., 0., 1.),
            color_to: Color::new_unpremul(0., 1., 0., 1.),
            color_from_inactive: Color::new_unpremul(1., 0., 0., 1.),
            color_to_inactive: Color::new_unpremul(0., 1., 0., 1.),
            fade: 1.,
            angle: 0.,
            geometry: Rectangle::from_size(area.size),
            border_width: 0.,
            corner_radius: CornerRadius::default(),
            scale: 1.,
            alpha: 1.,
        })
        .with_location(area.loc)]
        .into_iter()
        .map(|elem| Box::new(elem) as _)
        .collect()
    }
}

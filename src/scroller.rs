use bevy::{
  prelude::*,
  reflect::Reflect,
  render::{
    camera::{RenderTarget, Viewport},
    render_resource::{
      Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    },
    view::RenderLayers,
  },
};

#[derive(Reflect, Default, Debug, Clone)]
pub enum ScrollerDirection {
  #[default]
  Forward,
  Backward,
}

impl ScrollerDirection {
  pub fn as_f32(&self) -> f32 {
    (*self).clone().into()
  }
}

impl From<ScrollerDirection> for f32 {
  fn from(value: ScrollerDirection) -> Self {
    match value {
      ScrollerDirection::Forward => 1.,
      ScrollerDirection::Backward => -1.,
    }
  }
}
#[derive(Copy, Clone, Component)]
pub struct ScrollerItem {
  pub size: Vec2,
  pub parent: Entity,
}

#[derive(Copy, Clone, Component, Reflect)]
pub struct ScrollerSize {
  pub size: Vec2,
}

#[derive(Component)]
pub struct ScrollerInitialized;

#[derive(Default, Debug, Component, Clone, Reflect)]
#[reflect(Component)]
pub struct Scroller {
  pub start: f32,
  pub end: f32,
  pub speed: f32,
  pub direction: ScrollerDirection,
  pub is_paused: bool,
  pub spawn_edge: f32,
  pub render_layer: Option<u8>,
  pub texture_handle: Handle<Image>,
}

impl Scroller {
  pub fn new_item_needed(&self) -> bool {
    (self.start - self.spawn_edge) * self.direction.as_f32() < self.speed * 3.
  }

  pub fn get_next_item_position(&self, item: &ScrollerItem) -> Vec2 {
    Vec2 {
      x: self.spawn_edge - item.size.x / 2. * self.direction.as_f32(),
      ..default()
    }
  }
}

#[derive(Bundle)]
pub struct ScrollerBundle {
  pub name: Name,
  pub scroller: Scroller,
}

impl Default for ScrollerBundle {
  fn default() -> Self {
    Self {
      name: "Scroller".into(),
      scroller: Scroller::default(),
    }
  }
}

pub fn init(
  mut commands: Commands,
  mut q_scroller: Query<
    (Entity, &mut Scroller, &ScrollerSize, Option<&Transform>),
    Without<Visibility>,
  >,
  mut images: ResMut<Assets<Image>>,
) {
  for (scroller_entity, mut scroller, scroller_size, maybe_transform) in q_scroller.iter_mut() {
    scroller.end = scroller_size.size.x / 2. * scroller.direction.as_f32();
    scroller.start = -scroller.end;
    scroller.spawn_edge = scroller.end;
    let spatial = if let Some(transform) = maybe_transform {
      SpatialBundle::from_transform(*transform)
    } else {
      SpatialBundle::default()
    };
    commands.entity(scroller_entity).insert(spatial);
    if let Some(render_layer) = scroller.render_layer {
      let size = Extent3d {
        width: scroller_size.size.x as u32,
        height: scroller_size.size.y as u32,
        ..default()
      };

      let mut image = Image {
        texture_descriptor: TextureDescriptor {
          label: None,
          size,
          dimension: TextureDimension::D2,
          format: TextureFormat::Bgra8UnormSrgb,
          mip_level_count: 1,
          sample_count: 1,
          usage: TextureUsages::TEXTURE_BINDING
            | TextureUsages::COPY_DST
            | TextureUsages::RENDER_ATTACHMENT,
          view_formats: &[],
        },
        ..default()
      };

      image.resize(size);
      let image_handle = images.add(image); //  TODO: remove it on cleanup
      scroller.texture_handle = image_handle.clone();

      commands.entity(scroller_entity).with_children(|parent| {
        parent.spawn((
          Camera2dBundle {
            camera: Camera {
              viewport: Some(Viewport {
                physical_size: scroller_size.size.as_uvec2(),
                ..Default::default()
              }),
              order: -1,
              target: RenderTarget::Image(image_handle.clone()),
              ..default()
            },
            ..default()
          },
          RenderLayers::layer(render_layer),
          Name::new("Scroller Camera"),
        ));
        parent.spawn((
          SpriteBundle {
            texture: image_handle,
            ..Default::default()
          },
          Name::new("Scroller Camera texture"),
        ));
      });
    }
  }
}

pub fn wait_items(
  mut q_scroller: Query<(Entity, &Scroller, &mut Visibility), Without<ScrollerInitialized>>,
  mut commands: Commands,
) {
  for (entity, scroller, mut visibility) in q_scroller.iter_mut() {
    if !scroller.new_item_needed() {
      *visibility = Visibility::Inherited;
      commands.entity(entity).insert(ScrollerInitialized);
    }
  }
}

pub fn on_items_added(
  mut commands: Commands,
  mut q_added: Query<(&ScrollerItem, &mut Transform, &mut Visibility, Entity), Added<ScrollerItem>>,
  mut q_scroller: Query<(&mut Scroller, Entity)>,
) {
  for (scroller_item, mut transform, mut visibility, scroller_item_entity) in q_added.iter_mut() {
    if let Ok((mut scroller, scroller_entity)) = q_scroller.get_mut(scroller_item.parent) {
      let translation = scroller.get_next_item_position(scroller_item).extend(0.);

      transform.translation = translation;
      *visibility = Visibility::Inherited;
      if let Some(render_layer) = scroller.render_layer {
        commands
          .entity(scroller_item_entity)
          .insert(RenderLayers::layer(render_layer));
      }

      scroller.spawn_edge -= scroller_item.size.x * scroller.direction.as_f32();

      commands
        .entity(scroller_entity)
        .add_child(scroller_item_entity);
    }
  }
}

#[cfg(feature = "dev")]
pub fn scroller_debug(
  q_scroller_item: Query<(
    &GlobalTransform,
    &ScrollerItem,
    &Transform,
    Option<&Visibility>,
  )>,
  q_scroller: Query<(&GlobalTransform, &Scroller, &ScrollerSize)>,
  mut gizmos: Gizmos,
) {
  for (global_transfrorm, item, transform, visibility) in q_scroller_item.iter() {
    if let Some(visibility) = visibility {
      if visibility != Visibility::Hidden {
        let (scale, rotation, translation) = global_transfrorm.to_scale_rotation_translation();

        gizmos.rect_2d(
          translation.truncate(),
          rotation.to_axis_angle().1,
          item.size * scale.truncate(),
          Color::BLUE,
        );
      }
    }
  }
  for (global_transfrorm, scroller, scroller_size) in q_scroller.iter() {
    let (scale, rotation, translation) = global_transfrorm.to_scale_rotation_translation();

    gizmos.line_2d(
      Vec2::new(scroller.spawn_edge, scroller_size.size.y / -2. - 20.) * scale.truncate(), //  TODO: take rotation into account
      Vec2::new(scroller.spawn_edge, scroller_size.size.y / 2. + 20.) * scale.truncate(), //  TODO: take rotation into account
      Color::RED,
    );
    gizmos.rect_2d(
      translation.truncate(),
      rotation.to_axis_angle().1,
      Vec2::new(scroller_size.size.x, scroller_size.size.y) * scale.truncate(),
      Color::GREEN,
    );
    // gizmos.line_2d(
    //   Vec2::new(position.x, scroller.rect.min.y),
    //   Vec2::new(position.x, scroller.rect.max.y),
    //   Color::WHITE,
    // );
  }
}

pub fn update(
  mut q_scroller: Query<(&mut Scroller, Entity), With<ScrollerInitialized>>,
  mut q_item: Query<(&mut Transform, Entity, &ScrollerItem)>,
) {
  //   let step: f32 = 1. / 60.;
  //   let delta = time.delta_seconds();

  //   if delta > 0. {
  // println!("========= {}", q_item.iter().count());
  for (mut scroller, scroller_entity) in q_scroller.iter_mut() {
    if !scroller.new_item_needed() && !scroller.is_paused {
      scroller.spawn_edge += scroller.speed * scroller.direction.as_f32();
      q_item
        .iter_mut()
        .filter(|(_, _, item)| item.parent == scroller_entity)
        .for_each(|(mut transform, _, _)| {
          transform.translation +=
            Vec2::from([scroller.speed * scroller.direction.as_f32(), 0.]).extend(0.);
        })
    }
  }
  // let update_step = delta / step * scroller.speed;
  // let update_step = scroller.speed;
  // trace!("update_step: {update_step}, delta: {delta}");
  // println!(
  //   "current translation of {:?} is : {}",
  //   entity, container_transform.translation
  // );
  // }
}

pub fn delete_items(
  mut commands: Commands,
  q_scroller_item: Query<(&ScrollerItem, Entity, &Transform)>,
  q_scroller: Query<&Scroller, With<ScrollerInitialized>>,
) {
  for (scroller_item, entity, transform) in q_scroller_item.iter() {
    if let Ok(scroller) = q_scroller.get(scroller_item.parent) {
      if (scroller.end - transform.translation.x
        + scroller_item.size.x / 2. * scroller.direction.as_f32())
        * scroller.direction.as_f32()
        < 0.
      {
        commands.entity(entity).despawn_recursive();
      }
    }
  }
}

#[cfg(test)]
mod test {
  use crate::{Scroller, ScrollerItem};
  use bevy::prelude::*;
  use rstest::rstest;

  // #[rstest]
  // #[case(0., 30., Vec2::new(-5.0, 0.))]
  // #[case(10., 0., Vec2::new(15.0, 0.))]
  // #[case(65., 0., Vec2::new(-40.0, 0.))]
  // // #[case(65., 30., Vec2::new(25.0, 0.))]
  // fn get_inserted_item_position_test(
  //   #[case] end: f32,
  //   #[case] scroll: f32,
  //   #[case] expectation: Vec2,
  // ) {
  //   let scroller = Scroller {
  //     rect: Rect::from_center_size(Vec2::new(0., 0.), Vec2::new(80., 80.)),
  //     scroll,
  //     end,
  //     ..default()
  //   };
  //   // let position = scroller.get_next_item_position(&ScrollerItem { size: 30. });
  //   // assert_eq!(position, expectation);
  // }

  // #[rstest]
  // #[case(0., 30., true)]
  // #[case(10., 0., true)]
  // #[case(65., 0., true)]
  // fn should_add_new_item_test(#[case] end: f32, #[case] scroll: f32, #[case] expectation: bool) {
  //   let scroller = Scroller {
  //     rect: Rect::from_center_size(Vec2::new(0., 0.), Vec2::new(80., 80.)),
  //     scroll,
  //     end,
  //     ..default()
  //   };
  //   assert_eq!(scroller.should_insert_next_item(), expectation);
  // }
}

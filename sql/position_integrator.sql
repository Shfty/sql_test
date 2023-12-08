UPDATE component_position as p
SET x = px + vx,
    y = py + vy
FROM view_velocity_position as vp
where p.id = vp.id
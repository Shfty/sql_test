SELECT p.id,
    v.x AS vx,
    v.y AS vy,
    p.x AS px,
    p.y AS py
FROM component_position AS p
    INNER JOIN component_velocity AS v ON p.id = v.id
UPDATE component_velocity as v
SET x = CASE
        WHEN (
            bp.x < ?
            AND v.x < 0
        )
        OR (
            bp.x > ?
            AND v.x > 0
        ) THEN - v.x
        ELSE v.x
    END,
    y = CASE
        WHEN (
            bp.y < ?
            AND v.y < 0
        )
        OR (
            bp.y > ?
            AND v.y > 0
        ) THEN - v.y
        ELSE v.y
    END
FROM view_ball_position AS bp
WHERE v.id = bp.id
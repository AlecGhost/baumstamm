module Utils exposing (..)

import Element exposing (Color, rgb, toRgb)


select : (a -> Bool) -> (a -> b) -> (a -> b) -> a -> b
select condition f g x =
    if condition x then
        f x

    else
        g x


flip : (a -> b -> c) -> (b -> a -> c)
flip f =
    \b a -> f a b



{-
   Create an instance of `Color` from hsl values.

   See: https://www.baeldung.com/cs/convert-color-hsl-rgb
-}


hsl : Float -> Float -> Float -> Color
hsl h s l =
    let
        c =
            (1 - abs (2 * l - 1)) * s

        h1 =
            round (h / 60)

        x =
            c * toFloat (1 - abs (modBy 2 h1 - 1))

        m =
            l - c / 2

        ( r1, g1, b1 ) =
            if h1 <= 1 then
                ( c, x, 0 )

            else if h1 <= 2 then
                ( x, c, 0 )

            else if h1 <= 3 then
                ( 0, c, x )

            else if h1 <= 4 then
                ( 0, x, c )

            else if h1 <= 5 then
                ( x, 0, c )

            else if h1 <= 6 then
                ( c, 0, x )

            else
                ( 0, 0, 0 )

        ( r, g, b ) =
            ( r1 + m, g1 + m, b1 + m )
    in
    rgb r g b


toRgbString : Color -> String
toRgbString color =
    let
        { red, green, blue, alpha } =
            toRgb color
    in
    "rgba("
        ++ String.fromFloat (red * 100)
        ++ "%,"
        ++ String.fromFloat (green * 100)
        ++ "%,"
        ++ String.fromFloat (blue * 100)
        ++ "%,"
        ++ String.fromFloat (alpha * 100)
        ++ "%"
        ++ ")"

module Connections exposing (..)

import Data exposing (Connections, Crossing, Ending, Fraction, Orientation(..), Origin(..), Passing)
import Element exposing (Element, html)
import Svg
import Svg.Attributes as SAttr
import Utils exposing (..)


view : Connections -> Element msg
view connections =
    let
        percentile : Fraction -> String
        percentile fraction =
            let
                quotient =
                    (fraction.numerator |> toFloat) / (fraction.denominator |> toFloat)
            in
            quotient
                * 100
                |> String.fromFloat
                |> flip String.append "%"

        passing : Passing -> Svg.Svg msg
        passing p =
            Svg.line
                [ SAttr.x1 "0%"
                , SAttr.y1 <| percentile p.yFraction
                , SAttr.x2 "100%"
                , SAttr.y2 <| percentile p.yFraction
                , SAttr.stroke <| toRgbString p.color
                , SAttr.strokeWidth "2"
                ]
                []

        ending : Ending -> Svg.Svg msg
        ending e =
            let
                horizontalLine =
                    case e.origin of
                        Left ->
                            Svg.line
                                [ SAttr.x1 "0%"
                                , SAttr.y1 <| percentile e.yFraction
                                , SAttr.x2 <| percentile e.xFraction
                                , SAttr.y2 <| percentile e.yFraction
                                , SAttr.stroke <| toRgbString e.color
                                , SAttr.strokeWidth "2"
                                ]
                                []

                        Right ->
                            Svg.line
                                [ SAttr.x1 "100%"
                                , SAttr.y1 <| percentile e.yFraction
                                , SAttr.x2 <| percentile e.xFraction
                                , SAttr.y2 <| percentile e.yFraction
                                , SAttr.stroke <| toRgbString e.color
                                , SAttr.strokeWidth "2"
                                ]
                                []

                        None ->
                            Svg.line [] []

                verticalLine =
                    Svg.line
                        [ SAttr.x1 <| percentile e.xFraction
                        , SAttr.y1 <|
                            case connections.orientation of
                                Up ->
                                    "0%"

                                Down ->
                                    "100%"
                        , SAttr.x2 <| percentile e.xFraction
                        , SAttr.y2 <| percentile e.yFraction
                        , SAttr.stroke <| toRgbString e.color
                        , SAttr.strokeWidth "2"
                        ]
                        []
            in
            Svg.svg [] [ horizontalLine, verticalLine ]

        crossing : Crossing -> Svg.Svg msg
        crossing c =
            let
                horizontalLine =
                    case c.origin of
                        Left ->
                            Svg.line
                                [ SAttr.x1 "0%"
                                , SAttr.y1 <| percentile c.yFraction
                                , SAttr.x2 <| percentile c.xFraction
                                , SAttr.y2 <| percentile c.yFraction
                                , SAttr.stroke <| toRgbString c.color
                                , SAttr.strokeWidth "2"
                                ]
                                []

                        Right ->
                            Svg.line
                                [ SAttr.x1 "100%"
                                , SAttr.y1 <| percentile c.yFraction
                                , SAttr.x2 <| percentile c.xFraction
                                , SAttr.y2 <| percentile c.yFraction
                                , SAttr.stroke <| toRgbString c.color
                                , SAttr.strokeWidth "2"
                                ]
                                []

                        None ->
                            Svg.line [] []

                verticalLine =
                    Svg.line
                        [ SAttr.x1 <| percentile c.xFraction
                        , SAttr.y1 <|
                            case connections.orientation of
                                Up ->
                                    "100%"

                                Down ->
                                    "0%"
                        , SAttr.x2 <| percentile c.xFraction
                        , SAttr.y2 <| percentile c.yFraction
                        , SAttr.stroke <| toRgbString c.color
                        , SAttr.strokeWidth "2"
                        ]
                        []
            in
            Svg.svg [] [ horizontalLine, verticalLine ]
    in
    Svg.svg [ SAttr.width "100%", SAttr.height "100%", SAttr.pointerEvents "none" ]
        ((connections.passing |> List.map passing)
            ++ (connections.ending |> List.map ending)
            ++ (connections.crossing |> List.map crossing)
        )
        |> html

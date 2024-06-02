module Connections exposing (..)

import Element exposing (Color, Element, html)
import Svg
import Svg.Attributes as SAttr
import Utils exposing (..)


type alias Connections =
    { orientation : Orientation
    , totalX : Int
    , totalY : Int
    , passing : List Passing
    , ending : List Ending
    , crossing : List Crossing
    }


type alias Passing =
    { connection : Cid
    , color : Color
    , yIndex : Int
    }


type alias Ending =
    { connection : Cid
    , color : Color
    , origin : Origin
    , xIndex : Int
    , yIndex : Int
    }


type alias Crossing =
    { connection : Cid
    , color : Color
    , origin : Origin
    , xIndex : Int
    , yIndex : Int
    }


type Orientation
    = Up
    | Down


type Origin
    = Left
    | Right
    | None


type alias Cid =
    Int


connector : Connections -> Element msg
connector connections =
    let
        xOffset index =
            ((index + 1 |> toFloat) / (connections.totalX + 1 |> toFloat))
                * 100
                |> String.fromFloat
                |> flip String.append "%"

        yOffset index =
            ((index + 1 |> toFloat) / (connections.totalY + 1 |> toFloat))
                * 100
                |> String.fromFloat
                |> flip String.append "%"

        passing : Passing -> Svg.Svg msg
        passing p =
            Svg.line
                [ SAttr.x1 "0%"
                , SAttr.y1 <| yOffset p.yIndex
                , SAttr.x2 "100%"
                , SAttr.y2 <| yOffset p.yIndex
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
                                , SAttr.y1 <| yOffset e.yIndex
                                , SAttr.x2 <| xOffset e.xIndex
                                , SAttr.y2 <| yOffset e.yIndex
                                , SAttr.stroke <| toRgbString e.color
                                , SAttr.strokeWidth "2"
                                ]
                                []

                        Right ->
                            Svg.line
                                [ SAttr.x1 "100%"
                                , SAttr.y1 <| yOffset e.yIndex
                                , SAttr.x2 <| xOffset e.xIndex
                                , SAttr.y2 <| yOffset e.yIndex
                                , SAttr.stroke <| toRgbString e.color
                                , SAttr.strokeWidth "2"
                                ]
                                []

                        None ->
                            Svg.line [] []

                verticalLine =
                    Svg.line
                        [ SAttr.x1 <| xOffset e.xIndex
                        , SAttr.y1 <|
                            case connections.orientation of
                                Up ->
                                    "0%"

                                Down ->
                                    "100%"
                        , SAttr.x2 <| xOffset e.xIndex
                        , SAttr.y2 <| yOffset e.yIndex
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
                                , SAttr.y1 <| yOffset c.yIndex
                                , SAttr.x2 <| xOffset c.xIndex
                                , SAttr.y2 <| yOffset c.yIndex
                                , SAttr.stroke <| toRgbString c.color
                                , SAttr.strokeWidth "2"
                                ]
                                []

                        Right ->
                            Svg.line
                                [ SAttr.x1 "100%"
                                , SAttr.y1 <| yOffset c.yIndex
                                , SAttr.x2 <| xOffset c.xIndex
                                , SAttr.y2 <| yOffset c.yIndex
                                , SAttr.stroke <| toRgbString c.color
                                , SAttr.strokeWidth "2"
                                ]
                                []

                        None ->
                            Svg.line [] []

                verticalLine =
                    Svg.line
                        [ SAttr.x1 <| xOffset c.xIndex
                        , SAttr.y1 <|
                            case connections.orientation of
                                Up ->
                                    "100%"

                                Down ->
                                    "0%"
                        , SAttr.x2 <| xOffset c.xIndex
                        , SAttr.y2 <| yOffset c.yIndex
                        , SAttr.stroke <| toRgbString c.color
                        , SAttr.strokeWidth "2"
                        ]
                        []
            in
            Svg.svg [] [ horizontalLine, verticalLine ]
    in
    Svg.svg [ SAttr.width "100%", SAttr.height "100%" ]
        ((connections.passing |> List.map passing)
            ++ (connections.ending |> List.map ending)
            ++ (connections.crossing |> List.map crossing)
        )
        |> html

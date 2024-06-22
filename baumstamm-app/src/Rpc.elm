port module Rpc exposing (Incoming(..), Outgoing(..), decodeIncoming, encodeOutgoing, receive, send)

import Data exposing (..)
import Dict
import Json.Decode as Decode exposing (Value)
import Json.Encode as Encode
import Utils exposing (..)


port send : Value -> Cmd msg


port receive : (Value -> msg) -> Sub msg


type Outgoing
    = New
    | Load String
    | GetTreeData


encodeOutgoing : Outgoing -> Value
encodeOutgoing rpc =
    case rpc of
        New ->
            Encode.object
                [ ( "proc", Encode.string "new" )
                ]

        Load file ->
            Encode.object
                [ ( "proc", Encode.string "load" )
                , ( "payload", Encode.string file )
                ]

        GetTreeData ->
            Encode.object
                [ ( "proc", Encode.string "get_tree_data" )
                ]


type Incoming
    = TreeData Data.TreeData
    | InvalidProc
    | InvalidPayload


decodeIncoming : Value -> Incoming
decodeIncoming value =
    let
        procName =
            Decode.string

        proc =
            Decode.decodeValue (Decode.field "proc" procName) value
    in
    case proc of
        Ok "tree_data" ->
            let
                decodePersons =
                    Decode.list <|
                        Decode.map2 Person
                            (Decode.field "id" Decode.string)
                            (Decode.field "info"
                                (Decode.map (Maybe.withDefault Dict.empty)
                                    (Decode.maybe (Decode.dict Decode.string))
                                )
                            )

                decodeRelationships =
                    Decode.list <|
                        Decode.map3 Relationship
                            (Decode.field "id" Decode.string)
                            (Decode.field "parents"
                                (Decode.map2 Tuple.pair
                                    (Decode.index 0 (Decode.maybe Decode.string))
                                    (Decode.index 1 (Decode.maybe Decode.string))
                                )
                            )
                            (Decode.field "children" (Decode.list Decode.string))

                decodeColor =
                    Decode.map3 hsl
                        (Decode.index 0 Decode.float)
                        (Decode.index 1 Decode.float)
                        (Decode.index 2 Decode.float)

                decodeFraction =
                    Decode.map2 Fraction
                        (Decode.field "numerator" Decode.int)
                        (Decode.field "denominator" Decode.int)

                decodeOrientation =
                    Decode.string
                        |> Decode.andThen
                            (\str ->
                                case str of
                                    "Up" ->
                                        Decode.succeed Up

                                    "Down" ->
                                        Decode.succeed Down

                                    _ ->
                                        Decode.fail "Invalid orientation value"
                            )

                decodeOrigin =
                    Decode.string
                        |> Decode.andThen
                            (\str ->
                                case str of
                                    "Left" ->
                                        Decode.succeed Left

                                    "Right" ->
                                        Decode.succeed Right

                                    "None" ->
                                        Decode.succeed None

                                    _ ->
                                        Decode.fail "Invalid origin value"
                            )

                decodePassing =
                    Decode.map3 Passing
                        (Decode.field "rid" Decode.string)
                        (Decode.field "color" decodeColor)
                        (Decode.field "y_fraction" decodeFraction)

                decodeEnding =
                    Decode.map5 Ending
                        (Decode.field "rid" Decode.string)
                        (Decode.field "color" decodeColor)
                        (Decode.field "origin" decodeOrigin)
                        (Decode.field "x_fraction" decodeFraction)
                        (Decode.field "y_fraction" decodeFraction)

                decodeCrossing =
                    Decode.map5 Crossing
                        (Decode.field "rid" Decode.string)
                        (Decode.field "color" decodeColor)
                        (Decode.field "origin" decodeOrigin)
                        (Decode.field "x_fraction" decodeFraction)
                        (Decode.field "y_fraction" decodeFraction)

                decodeGrid =
                    Decode.list <|
                        Decode.list <|
                            Decode.oneOf
                                [ Decode.map PersonItem (Decode.field "Person" Decode.string)
                                , Decode.map ConnectionsItem
                                    (Decode.field "Connections"
                                        (Decode.map4
                                            Connections
                                            (Decode.field "orientation" decodeOrientation)
                                            (Decode.field "passing" (Decode.list decodePassing))
                                            (Decode.field "ending" (Decode.list decodeEnding))
                                            (Decode.field "crossing" (Decode.list decodeCrossing))
                                        )
                                    )
                                ]

                treeDataPayload =
                    Decode.map3 Data.TreeData
                        (Decode.field "persons" decodePersons)
                        (Decode.field "relationships" decodeRelationships)
                        (Decode.field "grid" decodeGrid)

                payload =
                    Decode.decodeValue (Decode.field "payload" treeDataPayload) value
            in
            case payload of
                Ok data ->
                    TreeData data

                _ ->
                    InvalidPayload

        _ ->
            InvalidProc

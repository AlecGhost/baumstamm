port module Rpc exposing (Incoming(..), Outgoing(..), decodeIncoming, encodeOutgoing, receive, send)

import Common
import Connections
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
    = TreeData Common.TreeData
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
                        Decode.map2 Common.Person
                            (Decode.field "id" Decode.string)
                            (Decode.field "info"
                                (Decode.map (Maybe.withDefault Dict.empty)
                                    (Decode.maybe (Decode.dict Decode.string))
                                )
                            )

                decodeRelationships =
                    Decode.list <|
                        Decode.map3 Common.Relationship
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

                decodeOrientation =
                    Decode.string
                        |> Decode.andThen
                            (\str ->
                                case str of
                                    "Up" ->
                                        Decode.succeed Connections.Up

                                    "Down" ->
                                        Decode.succeed Connections.Down

                                    _ ->
                                        Decode.fail "Invalid orientation value"
                            )

                decodeOrigin =
                    Decode.string
                        |> Decode.andThen
                            (\str ->
                                case str of
                                    "Left" ->
                                        Decode.succeed Connections.Left

                                    "Right" ->
                                        Decode.succeed Connections.Right

                                    "None" ->
                                        Decode.succeed Connections.None

                                    _ ->
                                        Decode.fail "Invalid origin value"
                            )

                decodePassing =
                    Decode.map3 Connections.Passing
                        (Decode.field "connection" Decode.int)
                        (Decode.field "color" decodeColor)
                        (Decode.field "y_index" Decode.int)

                decodeEnding =
                    Decode.map5 Connections.Ending
                        (Decode.field "connection" Decode.int)
                        (Decode.field "color" decodeColor)
                        (Decode.field "origin" decodeOrigin)
                        (Decode.field "x_index" Decode.int)
                        (Decode.field "y_index" Decode.int)

                decodeCrossing =
                    Decode.map5 Connections.Crossing
                        (Decode.field "connection" Decode.int)
                        (Decode.field "color" decodeColor)
                        (Decode.field "origin" decodeOrigin)
                        (Decode.field "x_index" Decode.int)
                        (Decode.field "y_index" Decode.int)

                decodeGrid =
                    Decode.list <|
                        Decode.list <|
                            Decode.oneOf
                                [ Decode.map Common.PersonItem (Decode.field "Person" Decode.string)
                                , Decode.map Common.ConnectionsItem
                                    (Decode.field "Connections"
                                        (Decode.map6
                                            Connections.Connections
                                            (Decode.field "orientation" decodeOrientation)
                                            (Decode.field "total_x" Decode.int)
                                            (Decode.field "total_y" Decode.int)
                                            (Decode.field "passing" (Decode.list decodePassing))
                                            (Decode.field "ending" (Decode.list decodeEnding))
                                            (Decode.field "crossing" (Decode.list decodeCrossing))
                                        )
                                    )
                                ]

                treeDataPayload =
                    Decode.map3 Common.TreeData
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

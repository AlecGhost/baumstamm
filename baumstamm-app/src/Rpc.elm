port module Rpc exposing (Incoming(..), Outgoing(..), decodeIncoming, encodeOutgoing, receive, send)

import Common
import Json.Decode as Decode exposing (Value)
import Json.Encode as Encode


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
                    Decode.value

                decodeRelationships =
                    Decode.value

                decodeGrid =
                    Decode.value

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

port module Rpc exposing (Incoming, Outgoing(..), decodeIncoming, encodeOutgoing, receive, send)

import Json.Decode as Decode exposing (Value)
import Json.Encode as Encode


port send : Value -> Cmd msg


port receive : (Value -> msg) -> Sub msg


type Outgoing
    = New
    | Load String
    | GetPersons


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

        GetPersons ->
            Encode.object
                [ ( "proc", Encode.string "get_persons" )
                ]


type Incoming
    = Persons Value
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
        Ok "persons" ->
            let
                personsPayload =
                    Decode.value

                payload =
                    Decode.decodeValue (Decode.field "payload" personsPayload) value
            in
            case payload of
                Ok persons ->
                    Persons persons

                _ ->
                    InvalidPayload

        _ ->
            InvalidProc

module Tree exposing (..)

import Common exposing (Settings)
import Connections
import Data exposing (GridItem(..), Pid, TreeData)
import Element exposing (..)
import Person


view :
    { treeData : TreeData
    , activePerson : Maybe Pid
    , onSelect : Pid -> msg
    , settings : Settings
    }
    -> Element msg
view { treeData, activePerson, onSelect, settings } =
    let
        viewItem : GridItem -> Element msg
        viewItem item =
            let
                h =
                    if settings.showProfilePictures then
                        300

                    else
                        100
            in
            el
                [ width (px 200)
                , height (px h)
                ]
                (case item of
                    PersonItem pid ->
                        let
                            isActive =
                                activePerson == Just pid
                        in
                        Person.view
                            { pid = pid
                            , isActive = isActive
                            , treeData = treeData
                            , onSelect = onSelect
                            , settings = settings
                            }

                    ConnectionsItem connections ->
                        Connections.view connections
                )

        viewRow : List GridItem -> Element msg
        viewRow r =
            row []
                (r |> List.map viewItem)

        viewColumn : List (List GridItem) -> Element msg
        viewColumn c =
            column []
                (c |> List.map viewRow)
    in
    viewColumn treeData.grid

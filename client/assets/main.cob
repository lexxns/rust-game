#manifest
self as example.client

#scenes

"game_container"
    AbsoluteNode{width:100vw height:100vh}
    "status"
        AbsoluteNode{top:5px right:9px bottom:Auto left:Auto}
        TextLine

    "turn_player"
        AbsoluteNode{top:50vh right:50vh}
        TextLine

    "button"
        AbsoluteNode{width:120px height:40px top:50vh right:50px left:Auto flex_direction:Column}
        Animated<BackgroundColor>{
            idle:Hsla{  hue:190 saturation:0.25 lightness:0.45 alpha:1.0 }
            hover:Hsla{ hue:120 saturation:1.0  lightness:0.50 alpha:1.0 }
            press:Hsla{ hue:240 saturation:1.0  lightness:0.50 alpha:1.0 }
        }
        TextLine{ text: "End Turn"}
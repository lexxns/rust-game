#manifest
self as example.client

#scenes
"scene"
    FlexNode{width:100vw height:100vh}

"game_container"
    FlexNode{width:100vw height:100vh flex_direction:Column justify_main:Center justify_cross:Center}
    "status"
        AbsoluteNode{top:5px right:9px bottom:Auto left:Auto}
        TextLine

    "owner"
        FlexNode{margin:{bottom:25px}}
        TextLine

    "button"
        FlexNode{width:300px height:175px}
        Multi<Responsive<BackgroundColor>>[
            {idle:Hsla{hue:190 saturation:0.25 lightness:0.45 alpha:1}}
            {state:[Selected] idle:Hsla{hue:125 saturation:0.4 lightness:0.3 alpha:1}}
        ]

"chat_container"
    AbsoluteNode{left:20px bottom:20px width:400px height:500px}
    FlexNode{
        flex_direction:Column
        padding:{top:10px bottom:10px left:10px right:10px}
        justify_main:FlexStart
        justify_cross:Stretch
    }
    Multi<Responsive<BackgroundColor>>[
        {idle:Hsla{hue:0 saturation:0 lightness:0 alpha:0.1}}
    ]

    "chat_history"
        FlexNode{
            height:400px
            width:380px
            margin:{bottom:10px}
            padding:{top:10px bottom:10px left:10px right:10px}
        }
        Multi<Responsive<BackgroundColor>>[
            {idle:Hsla{hue:192 saturation:0.25 lightness:0.85 alpha:0.5}}
        ]
        TextLine{
            text:""
            font:{family:"Fira Sans" width:Normal style:Normal weight:Medium}
            size:16
            linebreak:WordBoundary
            justify:Left
        }

    "chat_input"
        FlexNode{
            height:40px
            width:380px
            padding:{top:0px bottom:0px left:10px right:10px}
            justify_cross:Center
        }
        Multi<Responsive<BackgroundColor>>[
            {idle:Hsla{hue:192 saturation:0.25 lightness:0.85 alpha:0.5}}
        ]
        TextLine{
            text:"Chat Input"
            font:{family:"Fira Sans" width:Normal style:Normal weight:Medium}
            size:16
            linebreak:WordBoundary
            justify:Left
        }

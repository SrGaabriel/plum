let Plum = ./package.dhall

in  \(ctx : Plum.Context) ->
      { name = "example"
      , version = "1.0.0"
      , ghcOptions =
          if Plum.isLinux ctx then [ "-optl-pthread" ] else [] : List Text
      }

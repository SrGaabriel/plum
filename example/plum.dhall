let Plum = ./package.dhall

in  \(ctx : Plum.Context) ->
      { name = "test"
      , version = "1.0.0"
      , dependencies.megaparsec = "9.7.1"
      , ghcOptions =
          if Plum.isLinux ctx then [ "-optl-pthread" ] else [] : List Text
      }

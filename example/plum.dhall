let Plum = ./package.dhall

in  \(ctx : Plum.Context) ->
      { name = "test"
      , version = "1.0.0"
      , dependencies = {
        example = { path = "../example-lib" }
      }
      , ghcOptions =
          if Plum.isLinux ctx then [ "-optl-pthread" ] else [] : List Text
      }

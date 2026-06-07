let OS = < Linux | MacOS | Windows >

let Context = { os : OS, arch : Text, compiler : Text }

let Arch = { x86_64 = "x86_64", aarch64 = "aarch64" }

let isLinux =
      \(ctx : Context) ->
        merge { Linux = True, MacOS = False, Windows = False } ctx.os

let isMacos =
      \(ctx : Context) ->
        merge { Linux = False, MacOS = True, Windows = False } ctx.os

let isWindows =
      \(ctx : Context) ->
        merge { Linux = False, MacOS = False, Windows = True } ctx.os

in  { Context, OS, Arch, isLinux, isMacos, isWindows }

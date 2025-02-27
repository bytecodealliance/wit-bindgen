lazy val root = project
  .in(file("."))
  .settings(
    name := "wit-bindgen-scalajs-test",
    scalaVersion := "2.13.16",
  )
  .enablePlugins(ScalaJSPlugin)

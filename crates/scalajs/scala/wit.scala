package object wit {

  import scala.scalajs.js
  import scala.scalajs.js.JSConverters._
  import scala.scalajs.js.|

  sealed trait Nullable[+A] extends js.Any

  object Nullable {
    def some[A](value: A): Nullable[A] = value.asInstanceOf[Nullable[A]]

    val none: Nullable[Nothing] = null.asInstanceOf[Nullable[Nothing]]

    def fromOption[A](option: Option[A]): Nullable[A] =
      option match {
        case Some(value) => some(value)
        case None => none
      }
  }

  implicit class NullableOps[A](private val self: Nullable[A]) extends AnyVal {
    def toOption: Option[A] = Option(self.asInstanceOf[A])
  }

  sealed trait WitOption[A] extends js.Object {
    val tag: String
    val `val`: js.UndefOr[A]
  }

  object WitOption {
    def some[A](value: A): WitOption[A] = new WitOption[A] {
      val tag: String = "some"
      val `val`: js.UndefOr[A] = value
    }

    def none[A]: WitOption[A] = new WitOption[A] {
      val tag: String = "none"
      val `val`: js.UndefOr[A] = js.undefined
    }

    def fromOption[A](option: Option[A]): WitOption[A] =
      option match {
        case Some(value) => some(value)
        case None => none
      }
  }

  implicit class WitOptionOps[A](private val self: WitOption[A]) extends AnyVal {
    def toOption: Option[A] = self.tag match {
      case "some" => Some(self.`val`.get)
      case _ => None
    }
  }

  sealed trait WitResult[Ok, Err] extends js.Object {
    val tag: String
    val `val`: js.UndefOr[Ok | Err]
  }

  object WitResult {
    def ok[Ok, Err](value: Ok): WitResult[Ok, Err] = new WitResult[Ok, Err] {
      val tag: String = "ok"
      val `val`: js.UndefOr[Ok | Err] = value
    }

    def err[Ok, Err](value: Err): WitResult[Ok, Err] = new WitResult[Ok, Err] {
      val tag: String = "err"
      val `val`: js.UndefOr[Ok | Err] = value
    }

    def fromEither[E, A](either: Either[E, A]): WitResult[A, E] =
      either match {
        case Right(value) => ok(value)
        case Left(value) => err(value)
      }
  }

  implicit class WitResultOps[Ok, Err](private val self: WitResult[Ok, Err]) extends AnyVal {
    def toEither: Either[Err, Ok] = self.tag match {
      case "ok" => Right(self.`val`.get.asInstanceOf[Ok])
      case _ => Left(self.`val`.get.asInstanceOf[Err])
    }
  }

  type WitList[A] = js.Array[A]

  object WitList {
    def fromList[A](list: List[A]): WitList[A] = list.toJSArray
  }
}
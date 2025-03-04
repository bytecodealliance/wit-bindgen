package object wit {

  import scala.scalajs.js
  import scala.scalajs.js.JSConverters._
  import scala.scalajs.js.|
  import scala.scalajs.js.annotation.JSName

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

  sealed trait WitTuple0 extends js.Object {
  }

  object WitTuple0 {
    def apply(): WitTuple0 = js.Array().asInstanceOf[WitTuple0]

    def unapply(tuple: WitTuple0): Some[Unit] = Some(())

    implicit def fromScalaTuple0(tuple: Unit): WitTuple0 = WitTuple0()

    implicit def toScalaTuple0(tuple: WitTuple0): Unit = ()
  }

  sealed trait WitTuple1[T1] extends js.Object {
    @JSName("0") val _1: T1
  }

  object WitTuple1 {
    def apply[T1](_1: T1): WitTuple1[T1] = js.Array(_1).asInstanceOf[WitTuple1[T1]]

    def unapply[T1](tuple: WitTuple1[T1]): Some[(T1)] = Some(tuple)

    implicit def toScalaTuple1[T1](tuple: WitTuple1[T1]): (T1) = (tuple._1)
  }

  type WitTuple2[T1, T2] = js.Tuple2[T1, T2]
  type WitTuple3[T1, T2, T3] = js.Tuple3[T1, T2, T3]
  type WitTuple4[T1, T2, T3, T4] = js.Tuple4[T1, T2, T3, T4]
  type WitTuple5[T1, T2, T3, T4, T5] = js.Tuple5[T1, T2, T3, T4, T5]
  type WitTuple6[T1, T2, T3, T4, T5, T6] = js.Tuple6[T1, T2, T3, T4, T5, T6]
  type WitTuple7[T1, T2, T3, T4, T5, T6, T7] = js.Tuple7[T1, T2, T3, T4, T5, T6, T7]
  type WitTuple8[T1, T2, T3, T4, T5, T6, T7, T8] = js.Tuple8[T1, T2, T3, T4, T5, T6, T7, T8]
  type WitTuple9[T1, T2, T3, T4, T5, T6, T7, T8, T9] = js.Tuple9[T1, T2, T3, T4, T5, T6, T7, T8, T9]
  type WitTuple10[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10] = js.Tuple10[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10]
  type WitTuple11[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11] = js.Tuple11[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11]
  type WitTuple12[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12] = js.Tuple12[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12]
  type WitTuple13[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13] = js.Tuple13[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13]
  type WitTuple14[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14] = js.Tuple14[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14]
  type WitTuple15[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15] = js.Tuple15[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15]
  type WitTuple16[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16] = js.Tuple16[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16]
  type WitTuple17[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17] = js.Tuple17[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17]
  type WitTuple18[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18] = js.Tuple18[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18]
  type WitTuple19[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19] = js.Tuple19[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19]
  type WitTuple20[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20] = js.Tuple20[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20]
  type WitTuple21[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21] = js.Tuple21[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21]
  type WitTuple22[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22] = js.Tuple22[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22]
}
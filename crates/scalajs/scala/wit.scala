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

    implicit def fromScalaTuple1[T1](tuple: (T1)): WitTuple1[T1] = WitTuple1(tuple._1)

    implicit def toScalaTuple1[T1](tuple: WitTuple1[T1]): (T1) = (tuple._1)
  }

  sealed trait WitTuple2[T1, T2] extends js.Object {
    @JSName("0") val _1: T1
    @JSName("1") val _2: T2
  }

  object WitTuple2 {
    def apply[T1, T2](_1: T1, _2: T2): WitTuple2[T1, T2] = js.Array(_1, _2).asInstanceOf[WitTuple2[T1, T2]]

    def unapply[T1, T2](tuple: WitTuple2[T1, T2]): Some[(T1, T2)] = Some(tuple)

    implicit def fromScalaTuple2[T1, T2](tuple: (T1, T2)): WitTuple2[T1, T2] = WitTuple2(tuple._1, tuple._2)

    implicit def toScalaTuple2[T1, T2](tuple: WitTuple2[T1, T2]): (T1, T2) = (tuple._1, tuple._2)
  }

  sealed trait WitTuple3[T1, T2, T3] extends js.Object {
    @JSName("0") val _1: T1
    @JSName("1") val _2: T2
    @JSName("2") val _3: T3
  }

  object WitTuple3 {
    def apply[T1, T2, T3](_1: T1, _2: T2, _3: T3): WitTuple3[T1, T2, T3] = js.Array(_1, _2, _3).asInstanceOf[WitTuple3[T1, T2, T3]]

    def unapply[T1, T2, T3](tuple: WitTuple3[T1, T2, T3]): Some[(T1, T2, T3)] = Some(tuple)

    implicit def fromScalaTuple3[T1, T2, T3](tuple: (T1, T2, T3)): WitTuple3[T1, T2, T3] = WitTuple3(tuple._1, tuple._2, tuple._3)

    implicit def toScalaTuple3[T1, T2, T3](tuple: WitTuple3[T1, T2, T3]): (T1, T2, T3) = (tuple._1, tuple._2, tuple._3)
  }

  sealed trait WitTuple4[T1, T2, T3, T4] extends js.Object {
    @JSName("0") val _1: T1
    @JSName("1") val _2: T2
    @JSName("2") val _3: T3
    @JSName("3") val _4: T4
  }

  object WitTuple4 {
    def apply[T1, T2, T3, T4](_1: T1, _2: T2, _3: T3, _4: T4): WitTuple4[T1, T2, T3, T4] = js.Array(_1, _2, _3, _4).asInstanceOf[WitTuple4[T1, T2, T3, T4]]

    def unapply[T1, T2, T3, T4](tuple: WitTuple4[T1, T2, T3, T4]): Some[(T1, T2, T3, T4)] = Some(tuple)

    implicit def fromScalaTuple4[T1, T2, T3, T4](tuple: (T1, T2, T3, T4)): WitTuple4[T1, T2, T3, T4] = WitTuple4(tuple._1, tuple._2, tuple._3, tuple._4)

    implicit def toScalaTuple4[T1, T2, T3, T4](tuple: WitTuple4[T1, T2, T3, T4]): (T1, T2, T3, T4) = (tuple._1, tuple._2, tuple._3, tuple._4)
  }

  sealed trait WitTuple5[T1, T2, T3, T4, T5] extends js.Object {
    @JSName("0") val _1: T1
    @JSName("1") val _2: T2
    @JSName("2") val _3: T3
    @JSName("3") val _4: T4
    @JSName("4") val _5: T5
  }

  object WitTuple5 {
    def apply[T1, T2, T3, T4, T5](_1: T1, _2: T2, _3: T3, _4: T4, _5: T5): WitTuple5[T1, T2, T3, T4, T5] = js.Array(_1, _2, _3, _4, _5).asInstanceOf[WitTuple5[T1, T2, T3, T4, T5]]

    def unapply[T1, T2, T3, T4, T5](tuple: WitTuple5[T1, T2, T3, T4, T5]): Some[(T1, T2, T3, T4, T5)] = Some(tuple)

    implicit def fromScalaTuple5[T1, T2, T3, T4, T5](tuple: (T1, T2, T3, T4, T5)): WitTuple5[T1, T2, T3, T4, T5] = WitTuple5(tuple._1, tuple._2, tuple._3, tuple._4, tuple._5)

    implicit def toScalaTuple5[T1, T2, T3, T4, T5](tuple: WitTuple5[T1, T2, T3, T4, T5]): (T1, T2, T3, T4, T5) = (tuple._1, tuple._2, tuple._3, tuple._4, tuple._5)
  }

  sealed trait WitTuple6[T1, T2, T3, T4, T5, T6] extends js.Object {
    @JSName("0") val _1: T1
    @JSName("1") val _2: T2
    @JSName("2") val _3: T3
    @JSName("3") val _4: T4
    @JSName("4") val _5: T5
    @JSName("5") val _6: T6
  }

  object WitTuple6 {
    def apply[T1, T2, T3, T4, T5, T6](_1: T1, _2: T2, _3: T3, _4: T4, _5: T5, _6: T6): WitTuple6[T1, T2, T3, T4, T5, T6] = js.Array(_1, _2, _3, _4, _5, _6).asInstanceOf[WitTuple6[T1, T2, T3, T4, T5, T6]]

    def unapply[T1, T2, T3, T4, T5, T6](tuple: WitTuple6[T1, T2, T3, T4, T5, T6]): Some[(T1, T2, T3, T4, T5, T6)] = Some(tuple)

    implicit def fromScalaTuple6[T1, T2, T3, T4, T5, T6](tuple: (T1, T2, T3, T4, T5, T6)): WitTuple6[T1, T2, T3, T4, T5, T6] = WitTuple6(tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6)

    implicit def toScalaTuple6[T1, T2, T3, T4, T5, T6](tuple: WitTuple6[T1, T2, T3, T4, T5, T6]): (T1, T2, T3, T4, T5, T6) = (tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6)
  }

  sealed trait WitTuple7[T1, T2, T3, T4, T5, T6, T7] extends js.Object {
    @JSName("0") val _1: T1
    @JSName("1") val _2: T2
    @JSName("2") val _3: T3
    @JSName("3") val _4: T4
    @JSName("4") val _5: T5
    @JSName("5") val _6: T6
    @JSName("6") val _7: T7
  }

  object WitTuple7 {
    def apply[T1, T2, T3, T4, T5, T6, T7](_1: T1, _2: T2, _3: T3, _4: T4, _5: T5, _6: T6, _7: T7): WitTuple7[T1, T2, T3, T4, T5, T6, T7] = js.Array(_1, _2, _3, _4, _5, _6, _7).asInstanceOf[WitTuple7[T1, T2, T3, T4, T5, T6, T7]]

    def unapply[T1, T2, T3, T4, T5, T6, T7](tuple: WitTuple7[T1, T2, T3, T4, T5, T6, T7]): Some[(T1, T2, T3, T4, T5, T6, T7)] = Some(tuple)

    implicit def fromScalaTuple7[T1, T2, T3, T4, T5, T6, T7](tuple: (T1, T2, T3, T4, T5, T6, T7)): WitTuple7[T1, T2, T3, T4, T5, T6, T7] = WitTuple7(tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7)

    implicit def toScalaTuple7[T1, T2, T3, T4, T5, T6, T7](tuple: WitTuple7[T1, T2, T3, T4, T5, T6, T7]): (T1, T2, T3, T4, T5, T6, T7) = (tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7)
  }

  sealed trait WitTuple8[T1, T2, T3, T4, T5, T6, T7, T8] extends js.Object {
    @JSName("0") val _1: T1
    @JSName("1") val _2: T2
    @JSName("2") val _3: T3
    @JSName("3") val _4: T4
    @JSName("4") val _5: T5
    @JSName("5") val _6: T6
    @JSName("6") val _7: T7
    @JSName("7") val _8: T8
  }

  object WitTuple8 {
    def apply[T1, T2, T3, T4, T5, T6, T7, T8](_1: T1, _2: T2, _3: T3, _4: T4, _5: T5, _6: T6, _7: T7, _8: T8): WitTuple8[T1, T2, T3, T4, T5, T6, T7, T8] = js.Array(_1, _2, _3, _4, _5, _6, _7, _8).asInstanceOf[WitTuple8[T1, T2, T3, T4, T5, T6, T7, T8]]

    def unapply[T1, T2, T3, T4, T5, T6, T7, T8](tuple: WitTuple8[T1, T2, T3, T4, T5, T6, T7, T8]): Some[(T1, T2, T3, T4, T5, T6, T7, T8)] = Some(tuple)

    implicit def fromScalaTuple8[T1, T2, T3, T4, T5, T6, T7, T8](tuple: (T1, T2, T3, T4, T5, T6, T7, T8)): WitTuple8[T1, T2, T3, T4, T5, T6, T7, T8] = WitTuple8(tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8)

    implicit def toScalaTuple8[T1, T2, T3, T4, T5, T6, T7, T8](tuple: WitTuple8[T1, T2, T3, T4, T5, T6, T7, T8]): (T1, T2, T3, T4, T5, T6, T7, T8) = (tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8)
  }

  sealed trait WitTuple9[T1, T2, T3, T4, T5, T6, T7, T8, T9] extends js.Object {
    @JSName("0") val _1: T1
    @JSName("1") val _2: T2
    @JSName("2") val _3: T3
    @JSName("3") val _4: T4
    @JSName("4") val _5: T5
    @JSName("5") val _6: T6
    @JSName("6") val _7: T7
    @JSName("7") val _8: T8
    @JSName("8") val _9: T9
  }

  object WitTuple9 {
    def apply[T1, T2, T3, T4, T5, T6, T7, T8, T9](_1: T1, _2: T2, _3: T3, _4: T4, _5: T5, _6: T6, _7: T7, _8: T8, _9: T9): WitTuple9[T1, T2, T3, T4, T5, T6, T7, T8, T9] = js.Array(_1, _2, _3, _4, _5, _6, _7, _8, _9).asInstanceOf[WitTuple9[T1, T2, T3, T4, T5, T6, T7, T8, T9]]

    def unapply[T1, T2, T3, T4, T5, T6, T7, T8, T9](tuple: WitTuple9[T1, T2, T3, T4, T5, T6, T7, T8, T9]): Some[(T1, T2, T3, T4, T5, T6, T7, T8, T9)] = Some(tuple)

    implicit def fromScalaTuple9[T1, T2, T3, T4, T5, T6, T7, T8, T9](tuple: (T1, T2, T3, T4, T5, T6, T7, T8, T9)): WitTuple9[T1, T2, T3, T4, T5, T6, T7, T8, T9] = WitTuple9(tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9)

    implicit def toScalaTuple9[T1, T2, T3, T4, T5, T6, T7, T8, T9](tuple: WitTuple9[T1, T2, T3, T4, T5, T6, T7, T8, T9]): (T1, T2, T3, T4, T5, T6, T7, T8, T9) = (tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9)
  }

  sealed trait WitTuple10[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10] extends js.Object {
    @JSName("0") val _1: T1
    @JSName("1") val _2: T2
    @JSName("2") val _3: T3
    @JSName("3") val _4: T4
    @JSName("4") val _5: T5
    @JSName("5") val _6: T6
    @JSName("6") val _7: T7
    @JSName("7") val _8: T8
    @JSName("8") val _9: T9
    @JSName("9") val _10: T10
  }

  object WitTuple10 {
    def apply[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10](_1: T1, _2: T2, _3: T3, _4: T4, _5: T5, _6: T6, _7: T7, _8: T8, _9: T9, _10: T10): WitTuple10[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10] = js.Array(_1, _2, _3, _4, _5, _6, _7, _8, _9, _10).asInstanceOf[WitTuple10[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10]]

    def unapply[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10](tuple: WitTuple10[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10]): Some[(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)] = Some(tuple)

    implicit def fromScalaTuple10[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10](tuple: (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)): WitTuple10[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10] = WitTuple10(tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10)

    implicit def toScalaTuple10[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10](tuple: WitTuple10[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10]): (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10) = (tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10)
  }

  sealed trait WitTuple11[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11] extends js.Object {
    @JSName("0") val _1: T1
    @JSName("1") val _2: T2
    @JSName("2") val _3: T3
    @JSName("3") val _4: T4
    @JSName("4") val _5: T5
    @JSName("5") val _6: T6
    @JSName("6") val _7: T7
    @JSName("7") val _8: T8
    @JSName("8") val _9: T9
    @JSName("9") val _10: T10
    @JSName("10") val _11: T11
  }

  object WitTuple11 {
    def apply[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11](_1: T1, _2: T2, _3: T3, _4: T4, _5: T5, _6: T6, _7: T7, _8: T8, _9: T9, _10: T10, _11: T11): WitTuple11[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11] = js.Array(_1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11).asInstanceOf[WitTuple11[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11]]

    def unapply[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11](tuple: WitTuple11[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11]): Some[(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)] = Some(tuple)

    implicit def fromScalaTuple11[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11](tuple: (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)): WitTuple11[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11] = WitTuple11(tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11)

    implicit def toScalaTuple11[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11](tuple: WitTuple11[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11]): (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11) = (tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11)
  }

  sealed trait WitTuple12[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12] extends js.Object {
    @JSName("0") val _1: T1
    @JSName("1") val _2: T2
    @JSName("2") val _3: T3
    @JSName("3") val _4: T4
    @JSName("4") val _5: T5
    @JSName("5") val _6: T6
    @JSName("6") val _7: T7
    @JSName("7") val _8: T8
    @JSName("8") val _9: T9
    @JSName("9") val _10: T10
    @JSName("10") val _11: T11
    @JSName("11") val _12: T12
  }

  object WitTuple12 {
    def apply[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12](_1: T1, _2: T2, _3: T3, _4: T4, _5: T5, _6: T6, _7: T7, _8: T8, _9: T9, _10: T10, _11: T11, _12: T12): WitTuple12[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12] =
      js.Array(_1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12).asInstanceOf[WitTuple12[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12]]

    def unapply[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12](tuple: WitTuple12[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12]): Some[(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12)] =
      Some((tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11, tuple._12))

    implicit def fromScalaTuple12[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12](tuple: (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12)): WitTuple12[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12] =
      WitTuple12(tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11, tuple._12)

    implicit def toScalaTuple12[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12](tuple: WitTuple12[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12]): (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12) =
      (tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11, tuple._12)
  }

  sealed trait WitTuple13[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13] extends js.Object {
    @JSName("0") val _1: T1
    @JSName("1") val _2: T2
    @JSName("2") val _3: T3
    @JSName("3") val _4: T4
    @JSName("4") val _5: T5
    @JSName("5") val _6: T6
    @JSName("6") val _7: T7
    @JSName("7") val _8: T8
    @JSName("8") val _9: T9
    @JSName("9") val _10: T10
    @JSName("10") val _11: T11
    @JSName("11") val _12: T12
    @JSName("12") val _13: T13
  }

  object WitTuple13 {
    def apply[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13](_1: T1, _2: T2, _3: T3, _4: T4, _5: T5, _6: T6, _7: T7, _8: T8, _9: T9, _10: T10, _11: T11, _12: T12, _13: T13): WitTuple13[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13] =
      js.Array(_1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12, _13).asInstanceOf[WitTuple13[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13]]

    def unapply[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13](tuple: WitTuple13[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13]): Some[(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13)] =
      Some((tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11, tuple._12, tuple._13))

    implicit def fromScalaTuple13[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13](tuple: (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13)): WitTuple13[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13] =
      WitTuple13(tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11, tuple._12, tuple._13)

    implicit def toScalaTuple13[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13](tuple: WitTuple13[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13]): (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13) =
      (tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11, tuple._12, tuple._13)
  }

  sealed trait WitTuple14[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14] extends js.Object {
    @JSName("0") val _1: T1
    @JSName("1") val _2: T2
    @JSName("2") val _3: T3
    @JSName("3") val _4: T4
    @JSName("4") val _5: T5
    @JSName("5") val _6: T6
    @JSName("6") val _7: T7
    @JSName("7") val _8: T8
    @JSName("8") val _9: T9
    @JSName("9") val _10: T10
    @JSName("10") val _11: T11
    @JSName("11") val _12: T12
    @JSName("12") val _13: T13
    @JSName("13") val _14: T14
  }

  object WitTuple14 {
    def apply[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14](_1: T1, _2: T2, _3: T3, _4: T4, _5: T5, _6: T6, _7: T7, _8: T8, _9: T9, _10: T10, _11: T11, _12: T12, _13: T13, _14: T14): WitTuple14[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14] =
      js.Array(_1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12, _13, _14).asInstanceOf[WitTuple14[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14]]

    def unapply[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14](tuple: WitTuple14[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14]): Some[(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14)] =
      Some((tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11, tuple._12, tuple._13, tuple._14))

    implicit def fromScalaTuple14[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14](tuple: (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14)): WitTuple14[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14] =
      WitTuple14(tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11, tuple._12, tuple._13, tuple._14)

    implicit def toScalaTuple14[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14](tuple: WitTuple14[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14]): (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14) =
      (tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11, tuple._12, tuple._13, tuple._14)
  }

  sealed trait WitTuple15[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15] extends js.Object {
    @JSName("0") val _1: T1
    @JSName("1") val _2: T2
    @JSName("2") val _3: T3
    @JSName("3") val _4: T4
    @JSName("4") val _5: T5
    @JSName("5") val _6: T6
    @JSName("6") val _7: T7
    @JSName("7") val _8: T8
    @JSName("8") val _9: T9
    @JSName("9") val _10: T10
    @JSName("10") val _11: T11
    @JSName("11") val _12: T12
    @JSName("12") val _13: T13
    @JSName("13") val _14: T14
    @JSName("14") val _15: T15
  }

  object WitTuple15 {
    def apply[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15](_1: T1, _2: T2, _3: T3, _4: T4, _5: T5, _6: T6, _7: T7, _8: T8, _9: T9, _10: T10, _11: T11, _12: T12, _13: T13, _14: T14, _15: T15): WitTuple15[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15] =
      js.Array(_1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12, _13, _14, _15).asInstanceOf[WitTuple15[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15]]

    def unapply[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15](tuple: WitTuple15[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15]): Some[(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15)] =
      Some((tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11, tuple._12, tuple._13, tuple._14, tuple._15))

    implicit def fromScalaTuple15[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15](tuple: (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15)): WitTuple15[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15] =
      WitTuple15(tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11, tuple._12, tuple._13, tuple._14, tuple._15)

    implicit def toScalaTuple15[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15](tuple: WitTuple15[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15]): (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15) =
      (tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11, tuple._12, tuple._13, tuple._14, tuple._15)
  }

  sealed trait WitTuple16[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16] extends js.Object {
    @JSName("0") val _1: T1
    @JSName("1") val _2: T2
    @JSName("2") val _3: T3
    @JSName("3") val _4: T4
    @JSName("4") val _5: T5
    @JSName("5") val _6: T6
    @JSName("6") val _7: T7
    @JSName("7") val _8: T8
    @JSName("8") val _9: T9
    @JSName("9") val _10: T10
    @JSName("10") val _11: T11
    @JSName("11") val _12: T12
    @JSName("12") val _13: T13
    @JSName("13") val _14: T14
    @JSName("14") val _15: T15
    @JSName("15") val _16: T16
  }

  object WitTuple16 {
    def apply[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16](_1: T1, _2: T2, _3: T3, _4: T4, _5: T5, _6: T6, _7: T7, _8: T8, _9: T9, _10: T10, _11: T11, _12: T12, _13: T13, _14: T14, _15: T15, _16: T16): WitTuple16[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16] =
      js.Array(_1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12, _13, _14, _15, _16).asInstanceOf[WitTuple16[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16]]

    def unapply[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16](tuple: WitTuple16[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16]): Some[(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16)] =
      Some((tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11, tuple._12, tuple._13, tuple._14, tuple._15, tuple._16))

    implicit def fromScalaTuple16[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16](tuple: (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16)): WitTuple16[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16] =
      WitTuple16(tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11, tuple._12, tuple._13, tuple._14, tuple._15, tuple._16)

    implicit def toScalaTuple16[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16](tuple: WitTuple16[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16]): (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16) =
      (tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11, tuple._12, tuple._13, tuple._14, tuple._15, tuple._16)
  }

  sealed trait WitTuple17[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17] extends js.Object {
    @JSName("0") val _1: T1
    @JSName("1") val _2: T2
    @JSName("2") val _3: T3
    @JSName("3") val _4: T4
    @JSName("4") val _5: T5
    @JSName("5") val _6: T6
    @JSName("6") val _7: T7
    @JSName("7") val _8: T8
    @JSName("8") val _9: T9
    @JSName("9") val _10: T10
    @JSName("10") val _11: T11
    @JSName("11") val _12: T12
    @JSName("12") val _13: T13
    @JSName("13") val _14: T14
    @JSName("14") val _15: T15
    @JSName("15") val _16: T16
    @JSName("16") val _17: T17
  }

  object WitTuple17 {
    def apply[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17](_1: T1, _2: T2, _3: T3, _4: T4, _5: T5, _6: T6, _7: T7, _8: T8, _9: T9, _10: T10, _11: T11, _12: T12, _13: T13, _14: T14, _15: T15, _16: T16, _17: T17): WitTuple17[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17] =
      js.Array(_1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12, _13, _14, _15, _16, _17).asInstanceOf[WitTuple17[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17]]

    def unapply[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17](tuple: WitTuple17[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17]): Some[(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17)] =
      Some((tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11, tuple._12, tuple._13, tuple._14, tuple._15, tuple._16, tuple._17))

    implicit def fromScalaTuple17[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17](tuple: (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17)): WitTuple17[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17] =
      WitTuple17(tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11, tuple._12, tuple._13, tuple._14, tuple._15, tuple._16, tuple._17)

    implicit def toScalaTuple17[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17](tuple: WitTuple17[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17]): (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17) =
      (tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11, tuple._12, tuple._13, tuple._14, tuple._15, tuple._16, tuple._17)
  }

  sealed trait WitTuple18[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18] extends js.Object {
    @JSName("0") val _1: T1
    @JSName("1") val _2: T2
    @JSName("2") val _3: T3
    @JSName("3") val _4: T4
    @JSName("4") val _5: T5
    @JSName("5") val _6: T6
    @JSName("6") val _7: T7
    @JSName("7") val _8: T8
    @JSName("8") val _9: T9
    @JSName("9") val _10: T10
    @JSName("10") val _11: T11
    @JSName("11") val _12: T12
    @JSName("12") val _13: T13
    @JSName("13") val _14: T14
    @JSName("14") val _15: T15
    @JSName("15") val _16: T16
    @JSName("16") val _17: T17
    @JSName("17") val _18: T18
  }

  object WitTuple18 {
    def apply[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18](_1: T1, _2: T2, _3: T3, _4: T4, _5: T5, _6: T6, _7: T7, _8: T8, _9: T9, _10: T10, _11: T11, _12: T12, _13: T13, _14: T14, _15: T15, _16: T16, _17: T17, _18: T18): WitTuple18[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18] =
      js.Array(_1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12, _13, _14, _15, _16, _17, _18).asInstanceOf[WitTuple18[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18]]

    def unapply[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18](tuple: WitTuple18[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18]): Some[(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18)] =
      Some((tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11, tuple._12, tuple._13, tuple._14, tuple._15, tuple._16, tuple._17, tuple._18))

    implicit def fromScalaTuple18[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18](tuple: (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18)): WitTuple18[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18] =
      WitTuple18(tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11, tuple._12, tuple._13, tuple._14, tuple._15, tuple._16, tuple._17, tuple._18)

    implicit def toScalaTuple18[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18](tuple: WitTuple18[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18]): (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18) =
      (tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11, tuple._12, tuple._13, tuple._14, tuple._15, tuple._16, tuple._17, tuple._18)
  }

  sealed trait WitTuple19[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19] extends js.Object {
    @JSName("0") val _1: T1
    @JSName("1") val _2: T2
    @JSName("2") val _3: T3
    @JSName("3") val _4: T4
    @JSName("4") val _5: T5
    @JSName("5") val _6: T6
    @JSName("6") val _7: T7
    @JSName("7") val _8: T8
    @JSName("8") val _9: T9
    @JSName("9") val _10: T10
    @JSName("10") val _11: T11
    @JSName("11") val _12: T12
    @JSName("12") val _13: T13
    @JSName("13") val _14: T14
    @JSName("14") val _15: T15
    @JSName("15") val _16: T16
    @JSName("16") val _17: T17
    @JSName("17") val _18: T18
    @JSName("18") val _19: T19
  }

  object WitTuple19 {
    def apply[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19](_1: T1, _2: T2, _3: T3, _4: T4, _5: T5, _6: T6, _7: T7, _8: T8, _9: T9, _10: T10, _11: T11, _12: T12, _13: T13, _14: T14, _15: T15, _16: T16, _17: T17, _18: T18, _19: T19): WitTuple19[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19] =
      js.Array(_1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12, _13, _14, _15, _16, _17, _18, _19).asInstanceOf[WitTuple19[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19]]

    def unapply[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19](tuple: WitTuple19[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19]): Some[(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19)] =
      Some((tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11, tuple._12, tuple._13, tuple._14, tuple._15, tuple._16, tuple._17, tuple._18, tuple._19))

    implicit def fromScalaTuple19[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19](tuple: (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19)): WitTuple19[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19] =
      WitTuple19(tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11, tuple._12, tuple._13, tuple._14, tuple._15, tuple._16, tuple._17, tuple._18, tuple._19)

    implicit def toScalaTuple19[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19](tuple: WitTuple19[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19]): (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19) =
      (tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11, tuple._12, tuple._13, tuple._14, tuple._15, tuple._16, tuple._17, tuple._18, tuple._19)
  }

  sealed trait WitTuple20[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20] extends js.Object {
    @JSName("0") val _1: T1
    @JSName("1") val _2: T2
    @JSName("2") val _3: T3
    @JSName("3") val _4: T4
    @JSName("4") val _5: T5
    @JSName("5") val _6: T6
    @JSName("6") val _7: T7
    @JSName("7") val _8: T8
    @JSName("8") val _9: T9
    @JSName("9") val _10: T10
    @JSName("10") val _11: T11
    @JSName("11") val _12: T12
    @JSName("12") val _13: T13
    @JSName("13") val _14: T14
    @JSName("14") val _15: T15
    @JSName("15") val _16: T16
    @JSName("16") val _17: T17
    @JSName("17") val _18: T18
    @JSName("18") val _19: T19
    @JSName("19") val _20: T20
  }

  object WitTuple20 {
    def apply[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20](_1: T1, _2: T2, _3: T3, _4: T4, _5: T5, _6: T6, _7: T7, _8: T8, _9: T9, _10: T10, _11: T11, _12: T12, _13: T13, _14: T14, _15: T15, _16: T16, _17: T17, _18: T18, _19: T19, _20: T20): WitTuple20[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20] =
      js.Array(_1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12, _13, _14, _15, _16, _17, _18, _19, _20).asInstanceOf[WitTuple20[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20]]

    def unapply[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20](tuple: WitTuple20[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20]): Some[(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20)] =
      Some((tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11, tuple._12, tuple._13, tuple._14, tuple._15, tuple._16, tuple._17, tuple._18, tuple._19, tuple._20))

    implicit def fromScalaTuple20[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20](tuple: (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20)): WitTuple20[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20] =
      WitTuple20(tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11, tuple._12, tuple._13, tuple._14, tuple._15, tuple._16, tuple._17, tuple._18, tuple._19, tuple._20)

    implicit def toScalaTuple20[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20](tuple: WitTuple20[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20]): (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20) =
      (tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11, tuple._12, tuple._13, tuple._14, tuple._15, tuple._16, tuple._17, tuple._18, tuple._19, tuple._20)
  }

  sealed trait WitTuple21[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21] extends js.Object {
    @JSName("0") val _1: T1
    @JSName("1") val _2: T2
    @JSName("2") val _3: T3
    @JSName("3") val _4: T4
    @JSName("4") val _5: T5
    @JSName("5") val _6: T6
    @JSName("6") val _7: T7
    @JSName("7") val _8: T8
    @JSName("8") val _9: T9
    @JSName("9") val _10: T10
    @JSName("10") val _11: T11
    @JSName("11") val _12: T12
    @JSName("12") val _13: T13
    @JSName("13") val _14: T14
    @JSName("14") val _15: T15
    @JSName("15") val _16: T16
    @JSName("16") val _17: T17
    @JSName("17") val _18: T18
    @JSName("18") val _19: T19
    @JSName("19") val _20: T20
    @JSName("20") val _21: T21
  }

  object WitTuple21 {
    def apply[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21](_1: T1, _2: T2, _3: T3, _4: T4, _5: T5, _6: T6, _7: T7, _8: T8, _9: T9, _10: T10, _11: T11, _12: T12, _13: T13, _14: T14, _15: T15, _16: T16, _17: T17, _18: T18, _19: T19, _20: T20, _21: T21): WitTuple21[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21] =
      js.Array(_1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12, _13, _14, _15, _16, _17, _18, _19, _20, _21).asInstanceOf[WitTuple21[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21]]

    def unapply[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21](tuple: WitTuple21[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21]): Some[(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21)] =
      Some((tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11, tuple._12, tuple._13, tuple._14, tuple._15, tuple._16, tuple._17, tuple._18, tuple._19, tuple._20, tuple._21))

    implicit def fromScalaTuple21[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21](tuple: (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21)): WitTuple21[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21] =
      WitTuple21(tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11, tuple._12, tuple._13, tuple._14, tuple._15, tuple._16, tuple._17, tuple._18, tuple._19, tuple._20, tuple._21)

    implicit def toScalaTuple21[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21](tuple: WitTuple21[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21]): (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21) =
      (tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11, tuple._12, tuple._13, tuple._14, tuple._15, tuple._16, tuple._17, tuple._18, tuple._19, tuple._20, tuple._21)
  }

  sealed trait WitTuple22[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22] extends js.Object {
    @JSName("0") val _1: T1
    @JSName("1") val _2: T2
    @JSName("2") val _3: T3
    @JSName("3") val _4: T4
    @JSName("4") val _5: T5
    @JSName("5") val _6: T6
    @JSName("6") val _7: T7
    @JSName("7") val _8: T8
    @JSName("8") val _9: T9
    @JSName("9") val _10: T10
    @JSName("10") val _11: T11
    @JSName("11") val _12: T12
    @JSName("12") val _13: T13
    @JSName("13") val _14: T14
    @JSName("14") val _15: T15
    @JSName("15") val _16: T16
    @JSName("16") val _17: T17
    @JSName("17") val _18: T18
    @JSName("18") val _19: T19
    @JSName("19") val _20: T20
    @JSName("20") val _21: T21
    @JSName("21") val _22: T22
  }

  object WitTuple22 {
    def apply[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22](_1: T1, _2: T2, _3: T3, _4: T4, _5: T5, _6: T6, _7: T7, _8: T8, _9: T9, _10: T10, _11: T11, _12: T12, _13: T13, _14: T14, _15: T15, _16: T16, _17: T17, _18: T18, _19: T19, _20: T20, _21: T21, _22: T22): WitTuple22[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22] =
      js.Array(_1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12, _13, _14, _15, _16, _17, _18, _19, _20, _21, _22).asInstanceOf[WitTuple22[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22]]

    def unapply[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22](tuple: WitTuple22[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22]): Some[(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22)] =
      Some((tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11, tuple._12, tuple._13, tuple._14, tuple._15, tuple._16, tuple._17, tuple._18, tuple._19, tuple._20, tuple._21, tuple._22))

    implicit def fromScalaTuple22[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22](tuple: (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22)): WitTuple22[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22] =
      WitTuple22(tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11, tuple._12, tuple._13, tuple._14, tuple._15, tuple._16, tuple._17, tuple._18, tuple._19, tuple._20, tuple._21, tuple._22)

    implicit def toScalaTuple22[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22](tuple: WitTuple22[T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22]): (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22) =
      (tuple._1, tuple._2, tuple._3, tuple._4, tuple._5, tuple._6, tuple._7, tuple._8, tuple._9, tuple._10, tuple._11, tuple._12, tuple._13, tuple._14, tuple._15, tuple._16, tuple._17, tuple._18, tuple._19, tuple._20, tuple._21, tuple._22)
  }
}
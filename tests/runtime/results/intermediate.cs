namespace IntermediateWorld.wit.exports.test.results
{
    public class TestImpl : ITest
    {
        public static float StringError(float a)
        {
            return imports.test.results.TestInterop.StringError(a);
        }

        public static float EnumError(float a)
        {
            try {
                return imports.test.results.TestInterop.EnumError(a);
            } catch (WitException<imports.test.results.ITest.E> e) {
                throw new WitException(e.TypedValue, 0);
            }
        }

        public static float RecordError(float a)
        {
            try {
                return imports.test.results.TestInterop.RecordError(a);
            } catch (WitException<imports.test.results.ITest.E2> e) {
                throw new WitException(new ITest.E2(e.TypedValue.line, e.TypedValue.column), 0);
            }
        }

        public static float VariantError(float a)
        {
            try {
                return imports.test.results.TestInterop.VariantError(a);
            } catch (WitException<imports.test.results.ITest.E3> e)
                when (e.TypedValue.Tag == imports.test.results.ITest.E3.Tags.E1) {
                    throw new WitException(ITest.E3.E1((ITest.E)Enum.Parse(typeof(ITest.E), e.TypedValue.AsE1.ToString())), 0);
            } catch (WitException<imports.test.results.ITest.E3> e)
                when (e.TypedValue.Tag == imports.test.results.ITest.E3.Tags.E2) {
                    throw new WitException(ITest.E3.E2(new ITest.E2(e.TypedValue.AsE2.line, e.TypedValue.AsE2.column)), 0);
            }
            catch {
                throw new Exception("unreachable");
            }
        }

        public static uint EmptyError(uint a)
        {
            return imports.test.results.TestInterop.EmptyError(a);
        }

        public static void DoubleError(uint a)
        {
            imports.test.results.TestInterop.DoubleError(a);
        }
    }
}

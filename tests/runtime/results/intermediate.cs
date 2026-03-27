namespace IntermediateWorld.wit.Exports.test.results
{
    public class TestExportsImpl : ITestExports
    {
        public static float StringError(float a)
        {
            return Imports.test.results.ITestImports.StringError(a);
        }

        public static float EnumError(float a)
        {
            try {
                return Imports.test.results.ITestImports.EnumError(a);
            } catch (WitException<Imports.test.results.ITestImports.E> e) {
                throw new WitException(e.TypedValue, 0);
            }
        }

        public static float RecordError(float a)
        {
            try {
                return Imports.test.results.ITestImports.RecordError(a);
            } catch (WitException<Imports.test.results.ITestImports.E2> e) {
                throw new WitException(new ITestExports.E2(e.TypedValue.line, e.TypedValue.column), 0);
            }
        }

        public static float VariantError(float a)
        {
            try {
                return Imports.test.results.ITestImports.VariantError(a);
            } catch (WitException<Imports.test.results.ITestImports.E3> e)
                when (e.TypedValue.Tag == Imports.test.results.ITestImports.E3.Tags.E1) {
                    throw new WitException(ITestExports.E3.E1((ITestExports.E)Enum.Parse(typeof(ITestExports.E), e.TypedValue.AsE1.ToString())), 0);
            } catch (WitException<Imports.test.results.ITestImports.E3> e)
                when (e.TypedValue.Tag == Imports.test.results.ITestImports.E3.Tags.E2) {
                    throw new WitException(ITestExports.E3.E2(new ITestExports.E2(e.TypedValue.AsE2.line, e.TypedValue.AsE2.column)), 0);
            }
            catch {
                throw new Exception("unreachable");
            }
        }

        public static uint EmptyError(uint a)
        {
            return Imports.test.results.ITestImports.EmptyError(a);
        }

        public static void DoubleError(uint a)
        {
            Imports.test.results.ITestImports.DoubleError(a);
        }
    }
}

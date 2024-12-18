using ResultsWorld.wit.imports.test.results;

namespace ResultsWorld.wit.exports.test.results
{
    public class TestImpl : ITest
    {
        public static float StringError(float a)
        {
            return ResultsWorld.wit.imports.test.results.TestInterop.StringError(a);
        }

        public static float EnumError(float a)
        {
            try {
                return ResultsWorld.wit.imports.test.results.TestInterop.EnumError(a);
            } catch (ResultsWorld.wit.imports.test.results.ITest.EException e) {
                throw new WitException(e.EValue, 0);
            }
        }

        public static float RecordError(float a)
        {
            try {
                return ResultsWorld.wit.imports.test.results.TestInterop.RecordError(a);
            } catch (ResultsWorld.wit.imports.test.results.ITest.E2Exception e) {
                throw new WitException(new ITest.E2(e.E2Value.line, e.E2Value.column), 0);
            }
        }

        public static float VariantError(float a)
        {
            try {
                return ResultsWorld.wit.imports.test.results.TestInterop.VariantError(a);
            } catch (ResultsWorld.wit.imports.test.results.ITest.E3Exception e) 
                when (e.E3Value.Tag == ResultsWorld.wit.imports.test.results.ITest.E3.Tags.E1) {
                    throw new WitException(ITest.E3.E1((ITest.E)Enum.Parse(typeof(ITest.E), e.E3Value.AsE1.ToString())), 0);
            } catch (ResultsWorld.wit.imports.test.results.ITest.E3Exception e) 
                when (e.E3Value.Tag == ResultsWorld.wit.imports.test.results.ITest.E3.Tags.E2) {
                    throw new WitException(ITest.E3.E2(new ITest.E2(e.E3Value.AsE2.line, e.E3Value.AsE2.column)), 0);
            }
            catch {
                throw new Exception("unreachable");
            }
        }

        public static uint EmptyError(uint a)
        {
            return ResultsWorld.wit.imports.test.results.TestInterop.EmptyError(a);
        }

        public static void DoubleError(uint a)
        {
            ResultsWorld.wit.imports.test.results.TestInterop.DoubleError(a);
        }    
    }
}

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
            } catch (WitException e) {
                switch ((ResultsWorld.wit.imports.test.results.ITest.E) e.Value) {
                    case ResultsWorld.wit.imports.test.results.ITest.E.A:
                        throw new WitException(ITest.E.A, 0);
                    case ResultsWorld.wit.imports.test.results.ITest.E.B:
                        throw new WitException(ITest.E.B, 0);
                    case ResultsWorld.wit.imports.test.results.ITest.E.C:
                        throw new WitException(ITest.E.C, 0);
                    default:
                        throw new Exception("unreachable");
                }
            }
        }

        public static float RecordError(float a)
        {
            try {
                return ResultsWorld.wit.imports.test.results.TestInterop.RecordError(a);
            } catch (WitException e) {
                var value = (ResultsWorld.wit.imports.test.results.ITest.E2) e.Value;
                throw new WitException(new ITest.E2(value.line, value.column), 0);
            }
        }

        public static float VariantError(float a)
        {
            try {
                return ResultsWorld.wit.imports.test.results.TestInterop.VariantError(a);
            } catch (WitException e) {
                var value = (ResultsWorld.wit.imports.test.results.ITest.E3) e.Value;
                switch (value.Tag) {
                    case ResultsWorld.wit.imports.test.results.ITest.E3.E1:
                        switch (value.AsE1) {
                            case ResultsWorld.wit.imports.test.results.ITest.E.A:
                                throw new WitException(ITest.E3.e1(ITest.E.A), 0);
                            case ResultsWorld.wit.imports.test.results.ITest.E.B:
                                throw new WitException(ITest.E3.e1(ITest.E.B), 0);
                            case ResultsWorld.wit.imports.test.results.ITest.E.C:
                                throw new WitException(ITest.E3.e1(ITest.E.C), 0);
                            default:
                                throw new Exception("unreachable");
                        }
                    case ResultsWorld.wit.imports.test.results.ITest.E3.E2: {
                        throw new WitException(ITest.E3.e2(new ITest.E2(value.AsE2.line, value.AsE2.column)), 0);
                    }
                    default:
                        throw new Exception("unreachable");
                }
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

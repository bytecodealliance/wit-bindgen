//@ args = '--with-wit-results'

namespace IntermediateWorld.wit.exports.test.results
{
    public class TestImpl : ITest
    {
        public static Result<float, string> StringError(float a)
        {
            return imports.test.results.TestInterop.StringError(a);
        }

        public static Result<float, ITest.E> EnumError(float a)
        {
            var result = imports.test.results.TestInterop.EnumError(a);
            if (result.IsOk) {
                return Result<float, ITest.E>.Ok(result.AsOk);
            } else {
                switch (result.AsErr){
                    case imports.test.results.ITest.E.A:
                        return Result<float, ITest.E>.Err(ITest.E.A);
                    case imports.test.results.ITest.E.B:
                        return Result<float, ITest.E>.Err(ITest.E.B);
                    case imports.test.results.ITest.E.C:
                        return Result<float, ITest.E>.Err(ITest.E.C);
                    default:
                        throw new Exception("unreachable");
                }
            }
        }

        public static Result<float, ITest.E2> RecordError(float a)
        {
            var result = imports.test.results.TestInterop.RecordError(a);
            if (result.IsOk) {
                return Result<float, ITest.E2>.Ok(result.AsOk);
            } else {
                switch (result.AsErr) {
                    case imports.test.results.ITest.E2:
                        return Result<float, ITest.E2>.Err(new ITest.E2(result.AsErr.line, result.AsErr.column));
                    default:
                        throw new Exception("unreachable");
                }
            }
        }

        public static Result<float, ITest.E3> VariantError(float a)
        {
            var result = imports.test.results.TestInterop.VariantError(a);
            if (result.IsOk) {
                return Result<float, ITest.E3>.Ok(result.AsOk);
            } else {
                switch (result.AsErr) {
                    case imports.test.results.ITest.E3:
                        switch (result.AsErr.Tag){
                            case imports.test.results.ITest.E3.Tags.E1:
                                return Result<float, ITest.E3>.Err(ITest.E3.E1((ITest.E)Enum.Parse(typeof(ITest.E), result.AsErr.AsE1.ToString())));
                            case imports.test.results.ITest.E3.Tags.E2:
                                return  Result<float, ITest.E3>.Err(ITest.E3.E2(new ITest.E2(result.AsErr.AsE2.line, result.AsErr.AsE2.column)));
                            default:
                                throw new Exception("unreachable");
                        }
                    default:
                        throw new Exception("unreachable");
                }
            }
        }

        public static Result<uint, None> EmptyError(uint a)
        {
            return imports.test.results.TestInterop.EmptyError(a);
        }

        public static Result<Result<None, string>, string> DoubleError(uint a)
        {
            return imports.test.results.TestInterop.DoubleError(a);
        }
    }
}

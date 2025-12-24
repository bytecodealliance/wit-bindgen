//@ args = '--with-wit-results'
using IntermediateWorld.wit.Imports.test.results;

namespace IntermediateWorld.wit.Exports.test.results
{
    public class TestExportsImpl : ITestExports
    {
        public static Result<float, string> StringError(float a)
        {
            return ITestImports.StringError(a);
        }

        public static Result<float, ITestExports.E> EnumError(float a)
        {
            var result = ITestImports.EnumError(a);
            if (result.IsOk) {
                return Result<float, ITestExports.E>.Ok(result.AsOk);
            } else {
                switch (result.AsErr){
                    case ITestImports.E.A:
                        return Result<float, ITestExports.E>.Err(ITestExports.E.A);
                    case ITestImports.E.B:
                        return Result<float, ITestExports.E>.Err(ITestExports.E.B);
                    case ITestImports.E.C:
                        return Result<float, ITestExports.E>.Err(ITestExports.E.C);
                    default:
                        throw new Exception("unreachable");
                }
            }
        }

        public static Result<float, ITestExports.E2> RecordError(float a)
        {
            var result = ITestImports.RecordError(a);
            if (result.IsOk) {
                return Result<float, ITestExports.E2>.Ok(result.AsOk);
            } else {
                switch (result.AsErr) {
                    case ITestImports.E2:
                        return Result<float, ITestExports.E2>.Err(new ITestExports.E2(result.AsErr.line, result.AsErr.column));
                    default:
                        throw new Exception("unreachable");
                }
            }
        }

        public static Result<float, ITestExports.E3> VariantError(float a)
        {
            var result = ITestImports.VariantError(a);
            if (result.IsOk) {
                return Result<float, ITestExports.E3>.Ok(result.AsOk);
            } else {
                switch (result.AsErr) {
                    case ITestImports.E3:
                        switch (result.AsErr.Tag){
                            case ITestImports.E3.Tags.E1:
                                return Result<float, ITestExports.E3>.Err(ITestExports.E3.E1((ITestExports.E)Enum.Parse(typeof(ITestExports.E), result.AsErr.AsE1.ToString())));
                            case ITestImports.E3.Tags.E2:
                                return  Result<float, ITestExports.E3>.Err(ITestExports.E3.E2(new ITestExports.E2(result.AsErr.AsE2.line, result.AsErr.AsE2.column)));
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
            return ITestImports.EmptyError(a);
        }

        public static Result<Result<None, string>, string> DoubleError(uint a)
        {
            return ITestImports.DoubleError(a);
        }
    }
}

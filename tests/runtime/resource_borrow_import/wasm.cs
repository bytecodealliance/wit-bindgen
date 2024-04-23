using Import = ResourceBorrowImportWorld.wit.imports.test.resourceBorrowImport.ITest;
using Host = ResourceBorrowImportWorld.wit.imports.test.resourceBorrowImport.TestInterop;

namespace ResourceBorrowImportWorld
{
    public class ResourceBorrowImportWorldImpl : IResourceBorrowImportWorld {
	public static uint Test(uint v) {
	    return Host.Foo(new Import.Thing(v + 1)) + 4;
	}
    }
}

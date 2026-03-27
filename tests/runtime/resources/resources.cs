using System.Diagnostics;
using ResourcesWorld;
using ResourcesWorld.wit.Imports;

namespace ResourcesWorld.wit.Exports
{
    public class ExportsExportsImpl : IExportsExports
    {
        public static IExportsExports.Z Add(IExportsExports.Z a, IExportsExports.Z b)
        {
            return new Z(((Z) a).val + ((Z) b).val);
        }

        public static void Consume(IExportsExports.X x)
        {
            x.Dispose();
        }
        
        public static void TestImports()
        {
            var y1 = new IImportsImports.Y(10);
            Debug.Assert(y1.GetA() == 10);
	    y1.SetA(20);
            Debug.Assert(y1.GetA() == 20);	    
	    var y2 = IImportsImports.Y.Add(y1, 20);
            Debug.Assert(y2.GetA() == 40);

	    var y3 = new IImportsImports.Y(1);
	    var y4 = new IImportsImports.Y(2);
            Debug.Assert(y3.GetA() == 1);
            Debug.Assert(y4.GetA() == 2);
	    y3.SetA(10);
	    y4.SetA(20);	    
            Debug.Assert(y3.GetA() == 10);
            Debug.Assert(y4.GetA() == 20);	    	    
	    var y5 = IImportsImports.Y.Add(y3, 20);
	    var y6 = IImportsImports.Y.Add(y4, 30);	    
            Debug.Assert(y5.GetA() == 30);
            Debug.Assert(y6.GetA() == 50);
        }

        public class X : IExportsExports.X, IExportsExports.IX {
            public int val;

            public X(int val) {
                this.val = val;
            }

            public void SetA(int val) {
                this.val = val;
            }

            public int GetA() {
                return val;
            }

            public static IExportsExports.X Add(IExportsExports.X a, int b) {
                var myA = (X) a;
                myA.SetA(myA.GetA() + b);
                return myA;
            }
        }
    
        public class Z : IExportsExports.Z, IExportsExports.IZ {
            private static uint numDropped = 0;
            
            public int val;

            public Z(int val) {
                this.val = val;
            }

            public int GetA() {
                return val;
            }

            public static uint NumDropped() {
                return numDropped + 1;
            }

            override protected void Dispose(bool disposing) {
		numDropped += 1;
                
                base.Dispose(disposing);
            }
        }

        public class KebabCase : IExportsExports.KebabCase, IExportsExports.IKebabCase {
            public uint val;
            
            public KebabCase(uint val) {
                this.val = val;
            }
            
            public uint GetA() {
                return val;
            }

            public static uint TakeOwned(IExportsExports.KebabCase a) {
                return ((KebabCase) a).val;
            }
        }
    }
}

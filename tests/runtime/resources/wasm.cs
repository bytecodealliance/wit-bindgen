using System.Diagnostics;
using ResourcesWorld;
using ResourcesWorld.wit.imports;

namespace ResourcesWorld.wit.exports
{
    public class ExportsImpl : IExports
    {
        public static IExports.ZResource Add(IExports.ZResource a, IExports.ZResource b)
        {
            return new ZResource(((ZResource) a).val + ((ZResource) b).val);
        }

        public static void Consume(IExports.XResource x)
        {
            x.Dispose();
        }
        
        public static void TestImports()
        {
            var y1 = new IImports.YResource(10);
            Debug.Assert(y1.GetA() == 10);
	    y1.SetA(20);
            Debug.Assert(y1.GetA() == 20);	    
	    var y2 = IImports.YResource.Add(y1, 20);
            Debug.Assert(y2.GetA() == 40);

	    var y3 = new IImports.YResource(1);
	    var y4 = new IImports.YResource(2);
            Debug.Assert(y3.GetA() == 1);
            Debug.Assert(y4.GetA() == 2);
	    y3.SetA(10);
	    y4.SetA(20);	    
            Debug.Assert(y3.GetA() == 10);
            Debug.Assert(y4.GetA() == 20);	    	    
	    var y5 = IImports.YResource.Add(y3, 20);
	    var y6 = IImports.YResource.Add(y4, 30);	    
            Debug.Assert(y5.GetA() == 30);
            Debug.Assert(y6.GetA() == 50);
        }

        public class XResource : IExports.XResource, IExports.IXResource {
            public int val;

            public XResource(int val) {
                this.val = val;
            }

            public void SetA(int val) {
                this.val = val;
            }

            public int GetA() {
                return val;
            }

            public static IExports.XResource Add(IExports.XResource a, int b) {
                var myA = (XResource) a;
                myA.SetA(myA.GetA() + b);
                return myA;
            }
        }
    
        public class ZResource : IExports.ZResource, IExports.IZResource {
            private static uint numDropped = 0;
            
            public int val;

            public ZResource(int val) {
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

        public class KebabCaseResource : IExports.KebabCaseResource, IExports.IKebabCaseResource {
            public uint val;
            
            public KebabCaseResource(uint val) {
                this.val = val;
            }
            
            public uint GetA() {
                return val;
            }

            public static uint TakeOwned(IExports.KebabCaseResource a) {
                return ((KebabCaseResource) a).val;
            }
        }
    }
}

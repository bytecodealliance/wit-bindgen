using TestWorld.wit.Exports.test.resourceBorrowInRecord;

public class ToTestExportsImpl : IToTestExports {
    public class Thing : IToTestExports.Thing, IToTestExports.IThing {
        public string val;

        public Thing(string v) {
            this.val = v + " new";
        }

        public Thing(Thing other) {
            this.val = other.val + " test";
        }

        public string Get() {
            return val + " get";
        }
    }

    public static List<IToTestExports.Thing> Test(List<IToTestExports.Foo> v) {
        var myResult = new List<IToTestExports.Thing>();
        foreach (var foo in v)
        {
            myResult.Add(new Thing((Thing) foo.thing));
        }
        return myResult;
    }
}


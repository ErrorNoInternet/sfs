print("Loading SFS...", end='\r')

import sfs
import sys
import autorun

autorun.main()
sfs.init(sys.argv)

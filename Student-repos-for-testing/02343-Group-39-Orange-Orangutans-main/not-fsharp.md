# Why do we recommend F#?
* Functional programming languages are accepted to be the right choice for implementing compilers and interpreters.
* The concepts in the teaching material are often presented in a *declarative* way and not so often in *procedural* way. Hence, implementation in functional language is more natural while the gap to a procedural language is larger. 
* F# is the *official* functional programming language at DTU.
* We have run this course in the past in both F# and Java. F# projects were superior and students struggled less.
* Several students have used this project to learn F# in the past.
* Many students have reported that this course project provides a nice case study to learn F#.

# Alternatives

We have prepared a backup seting based on Java. 

First you need to replace the code folder with the Java starter. You can do it from the terminal as below

### macOS/Linux

```bash
rm -rf code
git clone git@github.com:team-checkr/java-starter.git code
rm -rf code/.git
git add code
git commit -m 'update code'
git push
```

### Windows

```powershell
rm -r -Force .\code
git clone git@github.com:team-checkr/java-starter.git code
rm -r -Force .\code\.git
git add code
git commit -m 'update code'
git push
```

Follow the instructions in the [code README.md](code/Readme.md).
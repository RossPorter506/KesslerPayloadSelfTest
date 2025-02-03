@echo off
where /q dslite.bat
if %errorlevel% neq 0 (
   echo Unable to find dslite.bat in your PATH. Assuming it's at .\uniflash\dslite.bat. If it isn't then this will fail.
   .\uniflash\dslite.bat --config=.\MSP430FR2355.ccxml -u %*
   exit /B %errorlevel%
)

dslite.bat --config=.\MSP430FR2355.ccxml -u %*


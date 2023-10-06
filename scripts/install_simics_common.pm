# © 2010 Intel Corporation
#
# This software and the related documents are Intel copyrighted materials, and
# your use of them is governed by the express license under which they were
# provided to you ("License"). Unless the License provides otherwise, you may
# not use, modify, copy, publish, distribute, disclose or transmit this software
# or the related documents without Intel's prior written permission.
#
# This software and the related documents are provided as is, with no express or
# implied warranties, other than those that are expressly stated in the License.

package install_simics_common;

use strict;
use warnings;
use English;
use Term::ReadLine;

# prevent readline warnings on Solaris perl 5.005.03
$ENV{PERL_READLINE_NOWARN} = "yes";

BEGIN {
    use Exporter ();
    use vars qw($VERSION @ISA @EXPORT @EXPORT_OK %EXPORT_TAGS);

    # set the version for version checking
    $VERSION     = 1.00;

    @ISA         = qw(Exporter);
    @EXPORT      = qw(&pr &log_to_file &pr_and_log &pr_columns
                      &strip &major_version &parse_packageinfo_file
                      &pkg_parse_packageinfo
                      &start_interactive_prompt &ask_question
                      &start_logging &stop_logging &version_cmp);
    %EXPORT_TAGS = ( );     # eg: TAG => [ qw!name1 name2! ],

    # your exported package globals go here,
    # as well as any optionally exported functions
    @EXPORT_OK   = qw($SIMICS_VERSION $SIMICS_PACKAGE_VERSION $LOGFILE $term $opt_batch);
}

use vars @EXPORT_OK;
use vars qw($SIMICS_VERSION $SIMICS_PACKAGE_VERSION $LOGFILE $term $opt_batch);

# simics version: limit what packages will be handled
$SIMICS_VERSION = '6';
$SIMICS_PACKAGE_VERSION = '6.0';

$term = 0;
$opt_batch = 0;

my $platform = $^O;
my $host = 'linux64';

###########
# logging #
###########

# print if batch mode disabled
sub pr {
    if (!$opt_batch) {
        print @_;
    }
}

# log to file
sub log_to_file {
    print $LOGFILE @_;
}

# print to the screen and log to file
sub pr_and_log {
    pr @_;
    log_to_file @_;
}

# print an array of strings as columns
sub pr_columns {
    my ($columns, $indent) = @_;
    my @sizes;
    foreach my $col (@$columns) {
        for (my $i = 0;  $i <= $#$col; $i++) {
            my $l = length($col->[$i]);
            if (!$sizes[$i] || ($l > $sizes[$i])) {
                $sizes[$i] = $l;
            }
        }
    }

    foreach my $col (@$columns) {
        if ($indent) {
            printf '%-' . $indent . 's', ' ';
        }
        for (my $i = 0; $i <= $#$col; $i++) {
            printf '%-' . ($sizes[$i] + 2) . 's', $col->[$i];
        }
        print "\n";
    }
}


#########
# utils #
#########

# strip spaces from the beginning and end of a string
sub strip {
    my ($str) = @_;
    $str =~ s/^\s*(.*?)\s*$/$1/;
    return $str;
}

# return the x.y out of a x.y.z version number
sub major_version {
    my ($version) = @_;
    if ($version =~ /^(\d+\.\d+[a-z]?)\./) {
        return $1;
    } else {
        return "";
    }
}


#####################
# Packageinfo files #
#####################

# parse a packageinfo file and return its contents as a hash
sub parse_packageinfo_file {
    my ($file) = @_;
    my %pkginfo;
    my @files;

    log_to_file "Parsing $file: ";
    if (open my $infile, "<", $file) {
        my $in_file = 0;
	while (<$infile>) {
            if ($in_file) {
                if (/^\s/) {
                    push @files, strip($_);
                } else {
                    $in_file = 0;
                }
            }
            if (!$in_file) {
                if (/^files:/) {
                    log_to_file "files ";
                    $in_file = 1;
                } elsif (/^([^\:]*):\s?(.*)$/) {
		    my ($kw, $val) = ($1, $2);
                    log_to_file "$kw ";
		    $kw =~ s/-/_/g;
                    $pkginfo{$kw} = $val;
                } else {
                    return 0;
                }
            }
        }
        close $infile;

        log_to_file "\n";
        $pkginfo{files} = \@files;
        return \%pkginfo;
    } else {
        log_to_file "failed to open file\n";
        return 0;
    }
}

sub pkg_parse_packageinfo {
    my ($pkg) = @_;

    log_to_file("Parsing packageinfo files for " . $pkg->{path} . "\n");
    my $pkginfo_dir = $pkg->{path} . "/packageinfo";
    if (opendir(my $dir, $pkginfo_dir)) {
        my @pkginfo_files = grep { !/^\./ && -f "$pkginfo_dir/$_" }
	                    readdir($dir);
	closedir $dir;
        if ($#pkginfo_files < 0) {
            pr_and_log("Ignoring " . $pkg->{path}
		       . ": empty packageinfo directory\n");
            return 0;
        }
        # here we assume that several packageinfo files mean several hosts, 
        # not that several packages are installed in the same directory 
	my $pkginfo;
	my @hosts;
        my $host_re = qr/$host$/;
        foreach my $pkginfo_file (@pkginfo_files) {
	    if ($pkginfo_file !~ $host_re) {
		# skip non-packageinfo files
		next;
	    }
		
            $pkginfo = parse_packageinfo_file("$pkginfo_dir/$pkginfo_file");
            if (!$pkginfo) {
                log_to_file("Error when parsing packaginfo file %pkginfo\n");
                return 0;
            }
            push @hosts, $pkginfo->{host};
        }
        $pkg->{packageinfo} = $pkginfo;
        $pkg->{hosts} = \@hosts;
        log_to_file("Package parsed successfully: "
		    . $pkg->{packageinfo}->{package_name} . "\n");
        return 1;
    } else {
        return 0;
    }
}


#####################
# Interactive usage #
#####################

# create a prompt
sub start_interactive_prompt {
    $term = new Term::ReadLine 'install-simics.pl', *STDIN, *STDOUT;
    $term->ornaments(0);
}

# Ask a question on the prompt
sub ask_question {
    my ($question, $default, @answers) = @_;

    $question .= " (";
    foreach my $answ (@answers) {
        $question .= $answ . ", ";
    }
    $question = substr($question, 0, -2) . ")";
    if ($default >= 0) {
        $question .= " [" . $answers[$default] . "]: ";
    } else {
        $question .= ": ";
    }
    log_to_file "question: $question";
    while (defined (my $answ = $term->readline($question))) {
        if ($answ) {
            $answ = lc($answ);
            foreach my $a (@answers) {
                if ($answ eq lc($a)) {
                    log_to_file $answ . "\n";
                    return $answ;
                }
            }
        } else {
            if ($default != -1) {
                log_to_file $answers[$default] . "\n";
                return $answers[$default];
            }
        }
    }
}

# start logging to file
sub start_logging {
    my ($log) = @_;
    if (!open($LOGFILE, ">", $log)) {
        if (!open($LOGFILE, ">", "/tmp/$log")) {
            open($LOGFILE, ">", "/dev/null");
        }
    }
}

sub stop_logging {
    close $LOGFILE;
    $LOGFILE = undef;
}

# compare two versions
sub version_cmp {
    my ($va, $vb) = @_;
    my @va = split /\./, $va;
    my @vb = split /\./, $vb;
    if ($va[2] =~ /pre(\d+)/) {
        $va[2] = -10000 + $1;
    }
    if ($vb[2] =~ /pre(\d+)/) {
        $vb[2] = -10000 + $1;
    }
    
    for (my $i = 0; $i < 3; $i++) {
        if ($va[$i] < $vb[$i]) {
            return -1;
        } elsif ($va[$i] > $vb[$i]) {
            return 1;
        }
    }
    return 0;
}

END { }

1;
